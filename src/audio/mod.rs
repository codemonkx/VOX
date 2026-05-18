use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::Result;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

pub struct Player {
    sink: Arc<Mutex<Option<Sink>>>,
    volume: Arc<Mutex<f32>>,
    muted: Arc<AtomicBool>,
    saved_volume: Arc<Mutex<f32>>,
    current_position: Arc<Mutex<f64>>,
    current_duration: Arc<Mutex<f64>>,
    is_playing: Arc<AtomicBool>,
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    position_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    current_path: Arc<Mutex<Option<String>>>,
    seek_offset: Arc<Mutex<f64>>,
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
            _stream,
            stream_handle,
            position_thread: Arc::new(Mutex::new(None)),
            current_path: Arc::new(Mutex::new(None)),
            seek_offset: Arc::new(Mutex::new(0.0)),
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

    pub fn play(&self, path: &Path, duration_override: Option<f64>) -> Result<()> {
        self.stop();

        *self.current_path.lock().unwrap() = Some(path.to_string_lossy().to_string());

        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;
        let duration = duration_override
            .or_else(|| source.total_duration().map(|d| d.as_secs_f64()))
            .unwrap_or(0.0);

        let sink = Sink::try_new(&self.stream_handle)?;

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
        // Strategy: try Sink::try_seek first (works for Seek-capable sources).
        // If that fails, fall back to recreating the source at the target position.
        {
            let lock = self.sink.lock().unwrap();
            if let Some(ref s) = *lock {
                if s.try_seek(Duration::from_secs_f64(seconds)).is_ok() {
                    *self.current_position.lock().unwrap() = seconds;
                    return;
                }
            }
        }

        // Fallback: restart playback at the target position
        let path_str = self.current_path.lock().unwrap().clone();
        let path_str = match path_str {
            Some(p) => p,
            None => return,
        };

        let dur = *self.current_duration.lock().unwrap();
        let target = seconds.min(dur).max(0.0);

        let file = match File::open(Path::new(&path_str)) {
            Ok(f) => f,
            Err(_) => return,
        };
        let source = match Decoder::new(BufReader::new(file)) {
            Ok(s) => s,
            Err(_) => return,
        };

        let was_paused = self.is_paused();
        let new_source = source.skip_duration(Duration::from_secs_f64(target));

        let sink = match Sink::try_new(&self.stream_handle) {
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
        // Stop the sink first so the position thread sees empty() and exits
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
    }

    pub fn set_volume(&self, vol: f32) {
        let vol = vol.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = vol;
        if !self.muted.load(Ordering::SeqCst) {
            let lock = self.sink.lock().unwrap();
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
        *self.saved_volume.lock().unwrap() = *self.volume.lock().unwrap();
        let lock = self.sink.lock().unwrap();
        if let Some(ref s) = *lock {
            s.set_volume(0.0);
        }
    }

    pub fn unmute(&self) {
        self.muted.store(false, Ordering::SeqCst);
        let vol = *self.saved_volume.lock().unwrap();
        *self.volume.lock().unwrap() = vol;
        let lock = self.sink.lock().unwrap();
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
}
