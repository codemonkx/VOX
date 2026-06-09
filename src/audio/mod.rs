use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

struct PositionTracker {
    inner: Box<dyn Source<Item = i16> + Send>,
    samples: Arc<AtomicU64>,
}

impl PositionTracker {
    fn new(inner: Box<dyn Source<Item = i16> + Send>, samples: Arc<AtomicU64>) -> Self {
        Self { inner, samples }
    }
}

impl Iterator for PositionTracker {
    type Item = i16;
    fn next(&mut self) -> Option<i16> {
        let s = self.inner.next()?;
        self.samples.fetch_add(1, Ordering::Relaxed);
        Some(s)
    }
}

impl Source for PositionTracker {
    fn current_frame_len(&self) -> Option<usize> { self.inner.current_frame_len() }
    fn channels(&self) -> u16 { self.inner.channels() }
    fn sample_rate(&self) -> u32 { self.inner.sample_rate() }
    fn total_duration(&self) -> Option<Duration> { self.inner.total_duration() }
}

pub struct Player {
    sink: Arc<Mutex<Option<Sink>>>,
    volume: Arc<Mutex<f32>>,
    muted: Arc<AtomicBool>,
    saved_volume: Arc<Mutex<f32>>,
    current_duration: Arc<Mutex<f64>>,
    is_playing: Arc<AtomicBool>,
    _stream: Mutex<Option<OutputStream>>,
    stream_handle: Mutex<Option<OutputStreamHandle>>,
    position_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    current_path: Arc<Mutex<Option<String>>>,
    seek_offset: Arc<Mutex<f64>>,
    current_rate: Arc<Mutex<u32>>,
    samples_consumed: Arc<AtomicU64>,
    channels: Arc<Mutex<u16>>,
    next_duration: Arc<Mutex<f64>>,
    prev_br_pos: Mutex<f64>,
    prev_br_time: Mutex<Instant>,
    last_vol_sync: Mutex<Instant>,
    seek_count: AtomicU64,
    current_seek_id: Arc<AtomicU64>,
    next_channels: Arc<Mutex<u16>>,
}

impl Player {
    pub fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        Ok(Self {
            sink: Arc::new(Mutex::new(None)),
            volume: Arc::new(Mutex::new(0.8)),
            muted: Arc::new(AtomicBool::new(false)),
            saved_volume: Arc::new(Mutex::new(0.8)),
            current_duration: Arc::new(Mutex::new(0.0)),
            is_playing: Arc::new(AtomicBool::new(false)),
            _stream: Mutex::new(Some(_stream)),
            stream_handle: Mutex::new(Some(stream_handle)),
            position_thread: Arc::new(Mutex::new(None)),
            current_path: Arc::new(Mutex::new(None)),
            seek_offset: Arc::new(Mutex::new(0.0)),
            current_rate: Arc::new(Mutex::new(0)),
            samples_consumed: Arc::new(AtomicU64::new(0)),
            channels: Arc::new(Mutex::new(2)),
            next_duration: Arc::new(Mutex::new(0.0)),
            prev_br_pos: Mutex::new(0.0),
            prev_br_time: Mutex::new(Instant::now()),
            last_vol_sync: Mutex::new(Instant::now()),
            seek_count: AtomicU64::new(0),
            current_seek_id: Arc::new(AtomicU64::new(0)),
            next_channels: Arc::new(Mutex::new(2)),
        })
    }

    fn start_position_thread(&self) {
        let is_playing = self.is_playing.clone();
        let sink_arc = self.sink.clone();
        let handle = thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(250));
                let lock = sink_arc.lock().unwrap();
                if let Some(ref s) = *lock {
                    if s.empty() {
                        is_playing.store(false, Ordering::SeqCst);
                        break;
                    }
                } else {
                    is_playing.store(false, Ordering::SeqCst);
                    break;
                }
            }
        });
        *self.position_thread.lock().unwrap() = Some(handle);
    }

    fn stop_thread(&self) {
        if let Some(h) = self.position_thread.lock().unwrap().take() {
            h.join().ok();
        }
    }

    pub fn play(&self, path: &Path, duration_override: Option<f64>, source_rate: u32) -> Result<()> {
        self.stop();

        *self.current_path.lock().unwrap() = Some(path.to_string_lossy().to_string());

        let is_dsf = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("dsf"))
            .unwrap_or(false);

        // Bit-perfect: open stream at source rate if device supports it.
        // Only recreates the stream when the rate actually changes.
        let old_rate = *self.current_rate.lock().unwrap();
        *self.current_rate.lock().unwrap() = source_rate;
        if !is_dsf && source_rate > 0 && old_rate != 0 && old_rate != source_rate
            && let Ok((s, h)) = create_stream_at_rate(source_rate) {
                *self._stream.lock().unwrap() = Some(s);
                *self.stream_handle.lock().unwrap() = Some(h.clone());
            }
        let handle = self.stream_handle.lock().unwrap().clone()
            .ok_or_else(|| anyhow::anyhow!("no stream handle"))?;

        let source: Box<dyn Source<Item = i16> + Send> = if is_dsf {
            let rate = current_device_rate();
            let output = std::process::Command::new("ffmpeg")
                .args(["-i", &path.to_string_lossy(), "-ar", &rate.to_string(), "-f", "wav", "-"])
                .output()
                .context("Failed to run ffmpeg for DSF decoding")?;

            if !output.status.success() {
                anyhow::bail!("ffmpeg failed to decode DSF file");
            }

            let mut wav_data = output.stdout;
            patch_wav_header(&mut wav_data);

            let source = Decoder::new(Cursor::new(wav_data))?;
            *self.channels.lock().unwrap() = source.channels();
            Box::new(PositionTracker::new(Box::new(source), self.samples_consumed.clone()))
        } else {
            let file = File::open(path)?;
            let source = Decoder::new(BufReader::new(file))?;
            *self.channels.lock().unwrap() = source.channels();
            Box::new(PositionTracker::new(Box::new(source), self.samples_consumed.clone()))
        };

        let duration = source.total_duration()
            .map(|d| d.as_secs_f64())
            .or(duration_override)
            .unwrap_or(0.0);

        if is_dsf {
            *self.current_rate.lock().unwrap() = current_device_rate();
        }

        let sink = Sink::try_new(&handle)?;

        // Volume handled by system mixer (wpctl), keep rodio at full
        sink.set_volume(1.0);

        sink.append(source);

        *self.current_duration.lock().unwrap() = duration;
        *self.next_duration.lock().unwrap() = 0.0;
        *self.sink.lock().unwrap() = Some(sink);
        self.samples_consumed.store(0, Ordering::Relaxed);
        *self.seek_offset.lock().unwrap() = 0.0;
        self.is_playing.store(true, Ordering::SeqCst);

        self.start_position_thread();

        Ok(())
    }

    pub fn seek(&self, seconds: f64) {
        let path_str = match self.current_path.lock().unwrap().clone() {
            Some(p) => p,
            None => return,
        };

        let dur = *self.current_duration.lock().unwrap();
        let target = seconds.min(dur).max(0.0);

        self.stop_thread();

        let mut old = self.sink.lock().unwrap();
        if let Some(s) = old.take() {
            s.stop();
        }
        drop(old);

        let is_dsf = Path::new(&path_str).extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("dsf"))
            .unwrap_or(false);

        let handle_arc = self.stream_handle.lock().unwrap().clone();
        let sink_arc = self.sink.clone();
        let seek_offset_arc = self.seek_offset.clone();
        let is_playing_arc = self.is_playing.clone();
        let channels_arc = self.channels.clone();
        let samples_consumed_arc = self.samples_consumed.clone();

        let seek_id = self.seek_count.fetch_add(1, Ordering::Relaxed) + 1;
        let current_seek_id = self.current_seek_id.clone();

        thread::spawn(move || {
            if seek_id != current_seek_id.load(Ordering::Relaxed) { return; }
            let source: Box<dyn Source<Item = i16> + Send> = {
                let mut cmd = std::process::Command::new("ffmpeg");
                cmd.args(["-ss", &target.to_string(), "-i", &path_str]);
                if is_dsf {
                    cmd.args(["-ar", &current_device_rate().to_string()]);
                }
                cmd.args(["-f", "wav", "-"]);

                let output = match cmd.output() {
                    Ok(o) if o.status.success() => o.stdout,
                    _ => { eprintln!("seek: ffmpeg failed for {path_str}"); return; }
                };

                let mut wav_data = output;
                patch_wav_header(&mut wav_data);

                let source = match Decoder::new(Cursor::new(wav_data)) {
                    Ok(d) => d,
                    Err(_) => { eprintln!("seek: Decoder::new failed"); return; }
                };
                *channels_arc.lock().unwrap() = source.channels();
                samples_consumed_arc.store(0, Ordering::Relaxed);
                Box::new(PositionTracker::new(Box::new(source), samples_consumed_arc))
            };

            if seek_id != current_seek_id.load(Ordering::Relaxed) { return; }

            let handle = match handle_arc {
                Some(h) => h,
                None => { eprintln!("seek: no stream handle"); return; }
            };

            let sink = match Sink::try_new(&handle) {
                Ok(s) => s,
                Err(_) => { eprintln!("seek: Sink::try_new failed"); return; }
            };

            sink.set_volume(1.0);
            sink.append(source);

            *sink_arc.lock().unwrap() = Some(sink);
            *seek_offset_arc.lock().unwrap() = target;
            is_playing_arc.store(true, Ordering::SeqCst);

            let pos_sink = sink_arc.clone();
            let pos_is_playing = is_playing_arc.clone();
            let pos_seek_id = current_seek_id.clone();
            thread::spawn(move || loop {
                if seek_id != pos_seek_id.load(Ordering::Relaxed) { return; }
                thread::sleep(Duration::from_millis(250));
                let lock = pos_sink.lock().unwrap();
                if let Some(ref s) = *lock {
                    if s.empty() {
                        pos_is_playing.store(false, Ordering::SeqCst);
                        break;
                    }
                } else {
                    pos_is_playing.store(false, Ordering::SeqCst);
                    break;
                }
            });
            current_seek_id.store(seek_id, Ordering::Relaxed);
        });
    }

    pub fn queue_next(&self, path: &Path, source_rate: u32) {
        if self.sink.lock().unwrap().is_none() {
            return;
        }
        let cur_rate = *self.current_rate.lock().unwrap();
        if source_rate != 0 && cur_rate != 0 && source_rate != cur_rate {
            eprintln!("queue_next: rate mismatch (cur={cur_rate}, next={source_rate}), skipping");
            return;
        }

        let path_buf = path.to_owned();
        let samples = self.samples_consumed.clone();
        let next_channels = self.next_channels.clone();
        let sink_arc = self.sink.clone();
        let next_dur = self.next_duration.clone();

        thread::spawn(move || {
            let is_dsf = path_buf.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("dsf"))
                .unwrap_or(false);

            let source: Box<dyn Source<Item = i16> + Send> = if is_dsf {
                let rate = current_device_rate();
                let output = match std::process::Command::new("ffmpeg")
                    .args(["-i", &path_buf.to_string_lossy(), "-ar", &rate.to_string(), "-f", "wav", "-"])
                    .output()
                {
                    Ok(o) if o.status.success() => o.stdout,
                    _ => { eprintln!("queue_next: ffmpeg failed for {path_buf:?}"); return; }
                };
                let mut wav_data = output;
                patch_wav_header(&mut wav_data);
                match Decoder::new(Cursor::new(wav_data)) {
                    Ok(d) => {
                        *next_channels.lock().unwrap() = d.channels();
                        *next_dur.lock().unwrap() = d.total_duration().map(|d| d.as_secs_f64()).unwrap_or(0.0);
                        Box::new(PositionTracker::new(Box::new(d), samples))
                    }
                    Err(_) => { eprintln!("queue_next: Decoder::new failed for DSF"); return; }
                }
            } else {
                let file = match File::open(&path_buf) {
                    Ok(f) => f,
                    Err(e) => { eprintln!("queue_next: open {path_buf:?}: {e}"); return; }
                };
                match Decoder::new(BufReader::new(file)) {
                    Ok(d) => {
                        *next_channels.lock().unwrap() = d.channels();
                        *next_dur.lock().unwrap() = d.total_duration().map(|d| d.as_secs_f64()).unwrap_or(0.0);
                        Box::new(PositionTracker::new(Box::new(d), samples))
                    }
                    Err(e) => { eprintln!("queue_next: Decoder::new failed: {e}"); return; }
                }
            };

            let lock = sink_arc.lock().unwrap();
            if let Some(ref sink) = *lock {
                sink.append(source);
            }
        });
    }

    pub fn pause(&self) {
        let lock = self.sink.lock().unwrap();
        if let Some(ref s) = *lock {
            s.pause();
        }
        self.is_playing.store(false, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        let lock = self.sink.lock().unwrap();
        if let Some(ref s) = *lock {
            s.play();
        }
        self.is_playing.store(true, Ordering::SeqCst);
    }

    pub fn stop(&self) {
        {
            let mut lock = self.sink.lock().unwrap();
            if let Some(s) = lock.take() {
                s.stop();
            }
        }

        self.stop_thread();

        self.is_playing.store(false, Ordering::SeqCst);
        *self.current_duration.lock().unwrap() = 0.0;
        *self.current_path.lock().unwrap() = None;
        *self.seek_offset.lock().unwrap() = 0.0;
        *self.current_rate.lock().unwrap() = 0;
        self.samples_consumed.store(0, Ordering::Relaxed);
        *self.channels.lock().unwrap() = 2;
    }

    pub fn set_volume(&self, vol: f32) {
        let vol = vol.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = vol;
        // Sync to system mixer (this is the actual output volume)
        Self::set_system_volume(vol);
    }

    pub fn read_system_volume() -> Option<f32> {
        let out = std::process::Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output().ok()?;
        if !out.status.success() { return None; }
        let s = std::str::from_utf8(&out.stdout).ok()?;
        let vol = s.trim_start_matches("Volume:")
            .trim().split_whitespace().next()?;
        vol.parse::<f32>().ok()
    }

    pub fn set_system_volume(vol: f32) {
        let vol = vol.clamp(0.0, 1.0);
        let out = std::process::Command::new("wpctl")
            .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{vol:.2}")])
            .output();
        if let Ok(o) = &out {
            if !o.status.success() {
                eprintln!("wpctl set-volume failed");
            }
        }
    }

    pub fn get_volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }

    /// Polls system volume via wpctl and updates internal tracking.
    /// Throttled to at most once every 200ms to avoid spawning too many processes.
    pub fn sync_volume_from_system(&self) {
        let mut last = self.last_vol_sync.lock().unwrap();
        if last.elapsed() < std::time::Duration::from_millis(200) {
            return;
        }
        *last = Instant::now();
        drop(last);
        if let Some(sys_vol) = Self::read_system_volume() {
            *self.volume.lock().unwrap() = sys_vol;
        }
    }

    pub fn mute(&self) {
        self.muted.store(true, Ordering::SeqCst);
        *self.saved_volume.lock().unwrap() = *self.volume.lock().unwrap();
        Self::set_system_volume(0.0);
    }

    pub fn unmute(&self) {
        self.muted.store(false, Ordering::SeqCst);
        let vol = *self.saved_volume.lock().unwrap();
        *self.volume.lock().unwrap() = vol;
        Self::set_system_volume(vol);
    }

    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::SeqCst)
    }

    pub fn current_position(&self) -> f64 {
        let samples = self.samples_consumed.load(Ordering::Relaxed);
        let ch = *self.channels.lock().unwrap();
        let rate = *self.current_rate.lock().unwrap();
        let pos = if ch > 0 && rate > 0 {
            samples as f64 / (ch as f64 * rate as f64)
        } else {
            0.0
        };
        let off = *self.seek_offset.lock().unwrap();
        pos + off
    }

    pub fn current_duration(&self) -> f64 {
        *self.current_duration.lock().unwrap()
    }

    pub fn take_next_duration(&self) -> f64 {
        let mut d = self.next_duration.lock().unwrap();
        let val = *d;
        *d = 0.0;
        val
    }

    pub fn take_next_channels(&self) -> u16 {
        let mut c = self.next_channels.lock().unwrap();
        let val = *c;
        *c = 0;
        val
    }

    pub fn is_paused(&self) -> bool {
        let lock = self.sink.lock().unwrap();
        lock.as_ref().map(|s| s.is_paused()).unwrap_or(false)
    }

    pub fn is_empty(&self) -> bool {
        let lock = self.sink.lock().unwrap();
        lock.as_ref().map(|s| s.empty()).unwrap_or(true)
    }

    pub fn realtime_bitrate(&self) -> u32 {
        let duration = *self.current_duration.lock().unwrap();
        if duration <= 0.0 {
            return 0;
        }
        let path = self.current_path.lock().unwrap().clone();
        let (_, avg) = match path {
            Some(ref p) => {
                if let Ok(meta) = std::fs::metadata(p) {
                    let bits = meta.len() * 8;
                    let avg = (bits as f64 / duration / 1000.0) as u32;
                    (bits, avg)
                } else {
                    return 0;
                }
            }
            None => return 0,
        };

        let pos = self.current_position();
        let now = Instant::now();
        let mut prev_pos = self.prev_br_pos.lock().unwrap();
        let mut prev_time = self.prev_br_time.lock().unwrap();
        let dt = now.duration_since(*prev_time).as_secs_f64();
        let dp = pos - *prev_pos;
        *prev_pos = pos;
        *prev_time = now;

        if dt < 0.5 || dp <= 0.0 {
            return avg;
        }
        let velocity = (dp / dt).clamp(0.5, 2.0);
        let inst = (avg as f64 * velocity) as u32;
        inst.max(avg.saturating_sub(100)).min(avg + 100)
    }
    pub fn set_duration(&self, dur: f64) {
        *self.current_duration.lock().unwrap() = dur;
    }

    pub fn adjust_seek_offset(&self, delta: f64) {
        *self.seek_offset.lock().unwrap() += delta;
    }

    pub fn set_channels(&self, ch: u16) {
        *self.channels.lock().unwrap() = ch;
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        if let Some(sink) = self.sink.lock().unwrap().take() {
            sink.stop();
        }
        if let Some(h) = self.position_thread.lock().unwrap().take() {
            h.join().ok();
        }
    }
}

fn current_device_rate() -> u32 {
    let host = rodio::cpal::default_host();
    if let Some(device) = host.default_output_device()
        && let Ok(config) = device.default_output_config() {
            return config.sample_rate().0;
        }
    48000
}

fn create_stream_at_rate(rate: u32) -> Result<(OutputStream, OutputStreamHandle)> {
    let host = rodio::cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("no output device"))?;

    let config = device
        .supported_output_configs()
        .map_err(|e| anyhow::anyhow!("{e:?}"))?
        .find(|c| {
            c.min_sample_rate().0 <= rate && c.max_sample_rate().0 >= rate
        })
        .map(|c| c.with_sample_rate(rodio::cpal::SampleRate(rate)))
        .ok_or_else(|| anyhow::anyhow!("rate {rate} not supported by device"))?;

    let (stream, handle) =
        OutputStream::try_from_device_config(&device, config)
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    Ok((stream, handle))
}

fn patch_wav_header(data: &mut [u8]) {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return;
    }

    let total_len = data.len() as u32;
    data[4..8].copy_from_slice(&(total_len - 8).to_le_bytes());

    let mut pos = 12;
    while pos + 8 <= data.len() {
        let chunk_size = u32::from_le_bytes([data[pos+4], data[pos+5], data[pos+6], data[pos+7]]);
        if &data[pos..pos+4] == b"data" && chunk_size == 0xFFFFFFFF {
            let actual_data = (data.len() - pos - 8) as u32;
            data[pos+4..pos+8].copy_from_slice(&actual_data.to_le_bytes());
            break;
        }
        pos += 8 + chunk_size as usize;
        if pos + 8 > data.len() {
            break;
        }
    }
}
