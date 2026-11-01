//! Audio-reactive "Music" software effect.
//!
//! Captures the default PipeWire output (sink monitor) with `pw-record`, computes
//! a smoothed RMS level, and exposes it to the GUI, which streams Static colour
//! frames to the keyboard through the normal verified `set_kbd_rgb` path.
//!
//! This is a Linux-native replacement for Armoury Crate's Windows-side Music
//! effect. It never touches the EC directly: every frame is a regular, clamped,
//! audited TUF RGB write.

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Pseudo effect id used by the GUI to mark the software Music effect.
/// It is never sent to the firmware; frames use Aura mode 0 (Static).
pub const MUSIC_EFFECT_ID: u32 = 0xFFFF_FF01;

pub struct MusicCapture {
    level: Arc<AtomicU32>,
    beat: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
    handle: Option<JoinHandle<()>>,
}

impl MusicCapture {
    /// Start capturing the default sink monitor. Returns an error string when
    /// pw-record is unavailable (the caller shows it in the status line).
    pub fn start() -> Result<Self, String> {
        let level = Arc::new(AtomicU32::new(0));
        let beat = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let child = Arc::new(Mutex::new(None));

        let spawn = |program: &str| {
            Command::new(program)
                .args([
                    "--target",
                    "auto",
                    "-P",
                    "stream.capture.sink=true",
                    "--rate",
                    "16000",
                    "--channels",
                    "1",
                    "--format",
                    "s16",
                    "-",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
        };
        let spawned = spawn("pw-record").or_else(|_| spawn("/usr/bin/pw-record"));
        let mut spawned = spawned.map_err(|e| format!("cannot start pw-record: {e}"))?;
        let mut out = spawned
            .stdout
            .take()
            .ok_or_else(|| "pw-record has no stdout".to_string())?;
        *child.lock().unwrap() = Some(spawned);

        let (lvl, bt, st) = (level.clone(), beat.clone(), stop.clone());
        let handle = std::thread::spawn(move || {
            let mut buf = [0u8; 640]; // 20 ms at 16 kHz mono s16
            let mut smoothed: u32 = 0;
            let mut avg: u32 = 0;
            while !st.load(Ordering::Relaxed) {
                match out.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let mut sum = 0.0f64;
                        let mut count = 0u32;
                        for ch in buf[..n].chunks_exact(2) {
                            let s = i16::from_le_bytes([ch[0], ch[1]]) as f64 / 32768.0;
                            sum += s * s;
                            count += 1;
                        }
                        let rms = if count > 0 {
                            (sum / count as f64).sqrt()
                        } else {
                            0.0
                        };
                        let target = rms_to_level(rms);
                        // fast attack, slow release — reads like a VU meter
                        smoothed = if target > smoothed {
                            (smoothed + target * 3) / 4
                        } else {
                            (smoothed * 7 + target) / 8
                        };
                        // ~0.4 s energy history for beat detection
                        avg = (avg * 15 + smoothed) / 16;
                        if target > 150
                            && (target as f32) > (avg as f32) * 1.35 + 30.0
                            && smoothed > 120
                        {
                            bt.store(true, Ordering::Relaxed);
                        }
                        lvl.store(smoothed, Ordering::Relaxed);
                    }
                }
            }
        });

        Ok(MusicCapture {
            level,
            beat,
            stop,
            child,
            handle: Some(handle),
        })
    }

    /// Smoothed level, 0..=1000.
    pub fn level(&self) -> u32 {
        self.level.load(Ordering::Relaxed)
    }

    /// Consume a pending beat flag (one flash per detected beat).
    pub fn take_beat(&self) -> bool {
        self.beat.swap(false, Ordering::Relaxed)
    }
}

impl Drop for MusicCapture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            let _ = child.kill();
        }
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            let _ = child.wait();
        }
    }
}

/// Map RMS amplitude (0..1) to a 0..1000 level on a -60..0 dBFS scale.
/// Pure so it can be unit-tested without audio hardware.
pub fn rms_to_level(rms: f64) -> u32 {
    let db = 20.0 * rms.max(1e-6).log10();
    (((db + 60.0) / 60.0).clamp(0.0, 1.0) * 1000.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_scale_is_monotonic_and_bounded() {
        assert_eq!(rms_to_level(0.0), 0);
        assert_eq!(rms_to_level(1.0), 1000);
        let quiet = rms_to_level(0.001);
        let mid = rms_to_level(0.03);
        let loud = rms_to_level(0.5);
        assert!(quiet < mid && mid < loud, "{quiet} {mid} {loud}");
    }
}
