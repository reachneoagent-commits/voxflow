//! Meeting mode: records your microphone and everything the Mac is playing
//! (the other side of a call) until stopped, then transcribes the mix like any
//! other recording and adds it to History.

#[cfg(target_os = "macos")]
mod system_audio_macos;
pub mod turns;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use handy_recorder::{AudioChunk, Recorder, RecorderConfig, Sink};
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Emitter, Manager};

use crate::audio_toolkit::save_wav_file;
use crate::managers::history::HistoryEntry;
use crate::managers::transcription_jobs::TranscriptionJobManager;

/// The speech models take 16 kHz audio.
const MODEL_RATE: u32 = 16_000;
/// Shorter than this is a mis-click, not a meeting.
const MIN_SECONDS: f32 = 3.0;

#[derive(Clone, Debug, Serialize, Type)]
pub struct MeetingStatus {
    pub recording: bool,
    pub seconds: u32,
    /// Whether the Mac's sound output is being recorded, or only the microphone.
    pub system_audio: bool,
}

/// Collects the microphone's audio and keeps how loud it is right now.
struct MicSink {
    samples: Vec<f32>,
    level: Arc<AtomicU32>,
    /// When the first audio arrived, to line this track up with the call's sound.
    first_audio: Arc<OnceLock<Instant>>,
}

impl Sink for MicSink {
    fn process_chunk(&mut self, chunk: AudioChunk<'_>) {
        self.first_audio.get_or_init(Instant::now);
        let real = &chunk.samples
            [..(chunk.valid_frames * chunk.channels as usize).min(chunk.samples.len())];
        let rms = (real.iter().map(|s| s * s).sum::<f32>() / real.len().max(1) as f32).sqrt();
        self.level.store(rms.to_bits(), Ordering::Relaxed);
        self.samples.extend_from_slice(real);
    }
}

/// What the meeting bar on screen shows, many times a second.
#[derive(Clone, Serialize)]
struct Tick {
    seconds: u32,
    mic: f32,
    system: f32,
    system_audio: bool,
}

/// How the meeting ended, for the bar's last message.
#[derive(Clone, Serialize)]
struct Ended {
    error: Option<String>,
}

struct Active {
    started: Instant,
    mic: Recorder<MicSink>,
    mic_level: Arc<AtomicU32>,
    mic_first_audio: Arc<OnceLock<Instant>>,
    #[cfg(target_os = "macos")]
    system: Option<system_audio_macos::SystemAudioCapture>,
}

#[derive(Default)]
pub struct MeetingManager {
    active: Mutex<Option<Active>>,
}

impl MeetingManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Active>> {
        self.active.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn status(&self) -> MeetingStatus {
        match &*self.lock() {
            Some(active) => MeetingStatus {
                recording: true,
                seconds: active.started.elapsed().as_secs() as u32,
                #[cfg(target_os = "macos")]
                system_audio: active.system.is_some(),
                #[cfg(not(target_os = "macos"))]
                system_audio: false,
            },
            None => MeetingStatus {
                recording: false,
                seconds: 0,
                system_audio: false,
            },
        }
    }

    pub fn start(self: &Arc<Self>, app: &AppHandle) -> Result<MeetingStatus, String> {
        let mut slot = self.lock();
        if slot.is_some() {
            return Err("A meeting is already being recorded".into());
        }

        let mic = Recorder::open(RecorderConfig::speech())
            .map_err(|e| format!("Could not open the microphone: {e}"))?;
        let mic_level = Arc::new(AtomicU32::new(0));
        let mic_first_audio = Arc::new(OnceLock::new());
        mic.start(MicSink {
            samples: Vec::new(),
            level: Arc::clone(&mic_level),
            first_audio: Arc::clone(&mic_first_audio),
        })
        .map_err(|e| format!("Could not start the microphone: {e}"))?;

        #[cfg(target_os = "macos")]
        let system = match system_audio_macos::SystemAudioCapture::start() {
            Ok(capture) => Some(capture),
            Err(e) => {
                // Still worth recording your own side.
                log::warn!("Meeting mode could not record system audio: {e}");
                None
            }
        };

        *slot = Some(Active {
            started: Instant::now(),
            mic,
            mic_level,
            mic_first_audio,
            #[cfg(target_os = "macos")]
            system,
        });
        drop(slot);
        crate::overlay::show_meeting_overlay(app);
        self.spawn_ticker(app);
        Ok(self.status())
    }

    /// Feeds the meeting bar until the meeting ends.
    fn spawn_ticker(self: &Arc<Self>, app: &AppHandle) {
        let manager = Arc::clone(self);
        let app = app.clone();
        std::thread::spawn(move || loop {
            let tick = match &*manager.lock() {
                Some(active) => Tick {
                    seconds: active.started.elapsed().as_secs() as u32,
                    mic: f32::from_bits(active.mic_level.load(Ordering::Relaxed)),
                    #[cfg(target_os = "macos")]
                    system: active.system.as_ref().map_or(0.0, |s| s.level()),
                    #[cfg(not(target_os = "macos"))]
                    system: 0.0,
                    #[cfg(target_os = "macos")]
                    system_audio: active.system.is_some(),
                    #[cfg(not(target_os = "macos"))]
                    system_audio: false,
                },
                None => break,
            };
            let _ = app.emit_to("recording_overlay", "meeting-tick", tick);
            std::thread::sleep(Duration::from_millis(100));
        });
    }

    /// Stops recording and returns the History entry being transcribed. The
    /// meeting bar says how it went, then goes away.
    pub fn stop(&self, app: &AppHandle) -> Result<HistoryEntry, String> {
        let result = self.finish(app);
        let _ = app.emit_to(
            "recording_overlay",
            "meeting-ended",
            Ended {
                error: result.as_ref().err().cloned(),
            },
        );
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(2500));
            crate::overlay::hide_recording_overlay(&app);
        });
        result
    }

    fn finish(&self, app: &AppHandle) -> Result<HistoryEntry, String> {
        let active = self.lock().take().ok_or("No meeting is being recorded")?;

        let mic = match active.mic.stop() {
            Ok(stopped) => stopped.sink.samples,
            Err(e) => return Err(format!("Could not stop the microphone: {e}")),
        };
        let _ = active.mic.close();

        #[cfg(target_os = "macos")]
        let (system, system_started) = match active.system {
            Some(capture) => {
                let rate = capture.sample_rate;
                let (samples, first) = capture.stop();
                (Some(to_model_rate(&samples, rate)), first)
            }
            None => (None, None),
        };
        #[cfg(not(target_os = "macos"))]
        let (system, system_started): (Option<Vec<f32>>, Option<Instant>) = (None, None);

        // The two sides start a moment apart (the system tap takes a while to
        // come up), so pad the later one so both share the meeting's clock.
        let (mic, system) = align(
            mic,
            system.unwrap_or_default(),
            active.mic_first_audio.get().copied(),
            system_started,
        );
        let system = Some(system);

        let mixed = mix(&mic, system.as_deref().unwrap_or(&[]));
        if (mixed.len() as f32 / MODEL_RATE as f32) < MIN_SECONDS {
            return Err("That recording was too short to keep".into());
        }

        let turns = turns::split(&mic, system.as_deref().unwrap_or(&[]));
        if turns.is_empty() {
            return Err("No speech was heard in that meeting".into());
        }

        let name = format!(
            "Meeting {}.wav",
            chrono::Local::now().format("%Y-%m-%d %H-%M")
        );
        let path = std::env::temp_dir().join(&name);
        save_wav_file(&path, &mixed).map_err(|e| e.to_string())?;
        let jobs = app.state::<Arc<TranscriptionJobManager>>();
        let entry = jobs
            .transcribe_meeting(path.clone(), turns)
            .map_err(|e| e.to_string());
        let _ = std::fs::remove_file(&path);
        entry
    }
}

/// Starts a meeting, or stops the one being recorded.
pub fn toggle(app: &AppHandle) {
    let manager = Arc::clone(&app.state::<Arc<MeetingManager>>());
    let app = app.clone();
    std::thread::spawn(move || {
        if manager.status().recording {
            match manager.stop(&app) {
                Ok(entry) => log::info!("Meeting saved as history entry {}", entry.id),
                Err(e) => log::warn!("Could not save the meeting: {e}"),
            }
        } else if let Err(e) = manager.start(&app) {
            log::warn!("Could not start the meeting: {e}");
        }
        crate::tray::update_tray_menu(&app);
    });
}

/// Averages `samples` down to 16 kHz. Speech survives this fine, and it keeps
/// the capture path free of a resampler.
fn to_model_rate(samples: &[f32], rate: u32) -> Vec<f32> {
    if rate == MODEL_RATE || rate == 0 {
        return samples.to_vec();
    }
    let ratio = rate as f64 / MODEL_RATE as f64;
    let out_len = (samples.len() as f64 / ratio) as usize;
    (0..out_len)
        .map(|i| {
            let start = (i as f64 * ratio) as usize;
            let end = (((i + 1) as f64 * ratio) as usize).clamp(start + 1, samples.len());
            let window = &samples[start..end];
            window.iter().sum::<f32>() / window.len() as f32
        })
        .collect()
}

/// Pads the side that started later with silence, so a sample at the same
/// position in both is from the same moment.
fn align(
    mut mic: Vec<f32>,
    mut system: Vec<f32>,
    mic_first: Option<Instant>,
    system_first: Option<Instant>,
) -> (Vec<f32>, Vec<f32>) {
    let (Some(mic_first), Some(system_first)) = (mic_first, system_first) else {
        return (mic, system);
    };
    if system_first >= mic_first {
        let pad = ((system_first - mic_first).as_secs_f64() * MODEL_RATE as f64) as usize;
        system.splice(0..0, std::iter::repeat(0.0).take(pad));
    } else {
        let pad = ((mic_first - system_first).as_secs_f64() * MODEL_RATE as f64) as usize;
        mic.splice(0..0, std::iter::repeat(0.0).take(pad));
    }
    (mic, system)
}

/// Adds the two sides together, scaled down only if they would clip.
fn mix(mic: &[f32], system: &[f32]) -> Vec<f32> {
    let len = mic.len().max(system.len());
    let mut out: Vec<f32> = (0..len)
        .map(|i| mic.get(i).copied().unwrap_or(0.0) + system.get(i).copied().unwrap_or(0.0))
        .collect();
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 0.99 {
        let scale = 0.99 / peak;
        out.iter_mut().for_each(|s| *s *= scale);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downsampling_48k_gives_a_third_of_the_samples() {
        let out = to_model_rate(&vec![0.5; 48_000], 48_000);
        assert_eq!(out.len(), 16_000);
        assert!(out.iter().all(|s| (s - 0.5).abs() < 1e-6));
    }

    #[test]
    fn mixing_pads_the_shorter_side_and_never_clips() {
        let out = mix(&[0.8, 0.8], &[0.8, 0.8, 0.8]);
        assert_eq!(out.len(), 3);
        assert!(out.iter().all(|s| s.abs() <= 0.99 + 1e-6));
        assert!(out[2] > 0.0);
    }

    #[test]
    fn the_side_that_started_later_is_padded_to_line_up() {
        let start = Instant::now();
        let later = start + Duration::from_millis(500);
        let (mic, system) = align(
            vec![1.0; 16_000],
            vec![1.0; 16_000],
            Some(start),
            Some(later),
        );
        assert_eq!(mic.len(), 16_000);
        assert_eq!(system.len(), 16_000 + 8_000);
        assert!(system[..8_000].iter().all(|s| *s == 0.0));
    }

    #[test]
    fn without_start_times_the_tracks_are_left_alone() {
        let (mic, system) = align(vec![1.0; 10], vec![1.0; 20], None, None);
        assert_eq!((mic.len(), system.len()), (10, 20));
    }

    #[test]
    fn quiet_audio_is_left_alone() {
        assert_eq!(mix(&[0.1, 0.2], &[0.1, 0.1]), vec![0.2, 0.3]);
    }
}
