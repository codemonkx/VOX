use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result};
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

pub struct Player {
    sink: Arc<Mutex<Option<Sink>>>,
    volume: Arc<Mutex<f32>>,
    muted: Arc<AtomicBool>,
    saved_volume: Arc<Mutex<f32>>,
    current_position: Arc<Mutex<f64>>,
    current_duration: Arc<Mutex<f64>>,
    is_playing: Arc<AtomicBool>,
    _stream: Mutex<Option<OutputStream>>,
    stream_handle: Mutex<Option<OutputStreamHandle>>,
    position_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    current_path: Arc<Mutex<Option<String>>>,
    seek_offset: Arc<Mutex<f64>>,
    current_rate: Arc<Mutex<u32>>,
    dsf_cache: Arc<Mutex<Option<Vec<u8>>>>,
}

impl Player {
    pub fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        Ok(Self {
            sink: Arc::new(Mutex::new(None)),
            volume: Arc::new(Mutex::new(0.8)),
            muted: Arc::new(AtomicBool::new(false)),
            saved_volume: Arc::new(Mutex::new(0.8)),
            current_position: Arc::new(Mutex::new(0.0)),
            current_duration: Arc::new(Mutex::new(0.0)),
            is_playing: Arc::new(AtomicBool::new(false)),
            _stream: Mutex::new(Some(_stream)),
            stream_handle: Mutex::new(Some(stream_handle)),
            position_thread: Arc::new(Mutex::new(None)),
            current_path: Arc::new(Mutex::new(None)),
            seek_offset: Arc::new(Mutex::new(0.0)),
            current_rate: Arc::new(Mutex::new(0)),
            dsf_cache: Arc::new(Mutex::new(None)),
        })
    }

    fn start_position_thread(&self) {
        let pos = self.current_position.clone();
        let is_playing = self.is_playing.clone();
        let sink_arc = self.sink.clone();
        let seek_off = self.seek_offset.clone();
        let handle = thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(250));
                let lock = sink_arc.lock().unwrap();
                if let Some(ref s) = *lock {
                    let off = *seek_off.lock().unwrap();
                    *pos.lock().unwrap() = s.get_pos().as_secs_f64() + off;
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
        *self.current_rate.lock().unwrap() = source_rate;

        let is_dsf = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("dsf"))
            .unwrap_or(false);

        if !is_dsf {
            *self.dsf_cache.lock().unwrap() = None;
        }

        // Bit-perfect: open stream at source rate if device supports it.
        // Only recreates the stream when the rate actually changes.
        if !is_dsf && source_rate > 0 && *self.current_rate.lock().unwrap() != source_rate {
            if let Ok((s, h)) = create_stream_at_rate(source_rate) {
                *self._stream.lock().unwrap() = Some(s);
                *self.stream_handle.lock().unwrap() = Some(h.clone());
            }
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

            *self.dsf_cache.lock().unwrap() = Some(wav_data.clone());
            Box::new(Decoder::new(Cursor::new(wav_data))?)
        } else {
            let file = File::open(path)?;
            Box::new(Decoder::new(BufReader::new(file))?)
        };

        let duration = source.total_duration()
            .map(|d| d.as_secs_f64())
            .or(duration_override)
            .unwrap_or(0.0);

        if is_dsf {
            *self.current_rate.lock().unwrap() = current_device_rate();
        }

        let sink = Sink::try_new(&handle)?;

        let vol = *self.volume.lock().unwrap();
        sink.set_volume(if self.muted.load(Ordering::SeqCst) {
            0.0
        } else {
            vol
        });

        sink.append(source);

        *self.current_duration.lock().unwrap() = duration;
        *self.sink.lock().unwrap() = Some(sink);
        *self.current_position.lock().unwrap() = 0.0;
        *self.seek_offset.lock().unwrap() = 0.0;
        self.is_playing.store(true, Ordering::SeqCst);

        self.start_position_thread();

        Ok(())
    }

    pub fn seek(&self, seconds: f64) {
        {
            let lock = self.sink.lock().unwrap();
            if let Some(ref s) = *lock {
                if s.try_seek(Duration::from_secs_f64(seconds)).is_ok() {
                    *self.current_position.lock().unwrap() = seconds;
                    return;
                }
            }
        }

        let path_str = self.current_path.lock().unwrap().clone();
        let path_str = match path_str {
            Some(p) => p,
            None => return,
        };

        let dur = *self.current_duration.lock().unwrap();
        let target = seconds.min(dur).max(0.0);

        let source: Box<dyn Source<Item = i16> + Send> = {
            let is_dsf = std::path::Path::new(&path_str)
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("dsf"))
                .unwrap_or(false);

            if is_dsf {
                let wav_data = match self.dsf_cache.lock().unwrap().clone() {
                    Some(wav) => wav,
                    None => {
                        let rate = current_device_rate();
                        let output = match std::process::Command::new("ffmpeg")
                            .args(["-i", &path_str, "-ar", &rate.to_string(), "-f", "wav", "-"])
                            .output()
                        {
                            Ok(o) => o,
                            Err(_) => return,
                        };
                        if !output.status.success() {
                            return;
                        }
                        output.stdout
                    }
                };
                let mut patched = wav_data;
                patch_wav_header(&mut patched);
                match Decoder::new(Cursor::new(patched)) {
                    Ok(d) => Box::new(d),
                    Err(_) => return,
                }
            } else {
                let file = match File::open(Path::new(&path_str)) {
                    Ok(f) => f,
                    Err(_) => return,
                };
                match Decoder::new(BufReader::new(file)) {
                    Ok(d) => Box::new(d),
                    Err(_) => return,
                }
            }
        };

        let was_paused = self.is_paused();
        let new_source = source.skip_duration(Duration::from_secs_f64(target));

        let handle = match self.stream_handle.lock().unwrap().clone() {
            Some(h) => h,
            None => return,
        };

        let sink = match Sink::try_new(&handle) {
            Ok(s) => s,
            Err(_) => return,
        };
        let vol = *self.volume.lock().unwrap();
        sink.set_volume(if self.muted.load(Ordering::SeqCst) {
            0.0
        } else {
            vol
        });
        sink.append(new_source);
        if was_paused {
            sink.pause();
        }

        self.stop_thread();

        let mut old = self.sink.lock().unwrap();
        if let Some(s) = old.take() {
            s.stop();
        }
        *old = Some(sink);
        drop(old);

        *self.current_position.lock().unwrap() = target;
        *self.seek_offset.lock().unwrap() = target;
        self.is_playing.store(true, Ordering::SeqCst);

        self.start_position_thread();
    }

    pub fn queue_next(&self, path: &Path, source_rate: u32) {
        if self.sink.lock().unwrap().is_none() {
            return;
        }
        let cur_rate = *self.current_rate.lock().unwrap();
        if source_rate != 0 && cur_rate != 0 && source_rate != cur_rate {
            return;
        }

        let is_dsf = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("dsf"))
            .unwrap_or(false);

        let source: Box<dyn Source<Item = i16> + Send> = if is_dsf {
            let rate = current_device_rate();
            let output = match std::process::Command::new("ffmpeg")
                .args(["-i", &path.to_string_lossy(), "-ar", &rate.to_string(), "-f", "wav", "-"])
                .output()
            {
                Ok(o) if o.status.success() => o.stdout,
                _ => return,
            };
            let mut wav_data = output;
            patch_wav_header(&mut wav_data);
            match Decoder::new(Cursor::new(wav_data)) {
                Ok(d) => Box::new(d),
                Err(_) => return,
            }
        } else {
            let file = match File::open(path) {
                Ok(f) => f,
                Err(_) => return,
            };
            match Decoder::new(BufReader::new(file)) {
                Ok(d) => Box::new(d),
                Err(_) => return,
            }
        };

        if let Some(ref sink) = *self.sink.lock().unwrap() {
            sink.append(source);
        }
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
        *self.current_position.lock().unwrap() = 0.0;
        *self.current_duration.lock().unwrap() = 0.0;
        *self.current_path.lock().unwrap() = None;
        *self.seek_offset.lock().unwrap() = 0.0;
        *self.current_rate.lock().unwrap() = 0;
        *self.dsf_cache.lock().unwrap() = None;
    }

    pub fn set_volume(&self, vol: f32) {
        let vol = vol.clamp(0.0, 1.0);
        let lock = self.sink.lock().unwrap();
        *self.volume.lock().unwrap() = vol;
        if !self.muted.load(Ordering::SeqCst) {
            if let Some(ref s) = *lock {
                s.set_volume(vol);
            }
        }
    }

    pub fn get_volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }

    pub fn mute(&self) {
        self.muted.store(true, Ordering::SeqCst);
        let lock = self.sink.lock().unwrap();
        *self.saved_volume.lock().unwrap() = *self.volume.lock().unwrap();
        if let Some(ref s) = *lock {
            s.set_volume(0.0);
        }
    }

    pub fn unmute(&self) {
        self.muted.store(false, Ordering::SeqCst);
        let lock = self.sink.lock().unwrap();
        let vol = *self.saved_volume.lock().unwrap();
        *self.volume.lock().unwrap() = vol;
        if let Some(ref s) = *lock {
            s.set_volume(vol);
        }
    }

    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::SeqCst)
    }

    pub fn current_position(&self) -> f64 {
        *self.current_position.lock().unwrap()
    }

    pub fn current_duration(&self) -> f64 {
        *self.current_duration.lock().unwrap()
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing.load(Ordering::SeqCst)
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
        match path {
            Some(p) => {
                if let Ok(meta) = std::fs::metadata(&p) {
                    let bits = meta.len() * 8;
                    (bits as f64 / duration / 1000.0) as u32
                } else {
                    0
                }
            }
            None => 0,
        }
    }
    pub fn set_duration(&self, dur: f64) {
        *self.current_duration.lock().unwrap() = dur;
    }

    pub fn adjust_seek_offset(&self, delta: f64) {
        *self.seek_offset.lock().unwrap() += delta;
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
    if let Some(device) = host.default_output_device() {
        if let Ok(config) = device.default_output_config() {
            return config.sample_rate().0;
        }
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
