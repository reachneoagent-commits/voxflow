//! Meeting mode: records your microphone and everything the Mac is playing
//! (the other side of a call) until stopped, then transcribes the mix like any
//! other recording and adds it to History.

#[cfg(target_os = "macos")]
mod system_audio_macos;

use std::sync::{Arc, Mutex};
use std::time::Instant;

use handy_recorder::{CollectingSink, Recorder, RecorderConfig};
use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Manager};

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

struct Active {
    started: Instant,
    mic: Recorder<CollectingSink>,
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

    pub fn start(&self) -> Result<MeetingStatus, String> {
        let mut slot = self.lock();
        if slot.is_some() {
            return Err("A meeting is already being recorded".into());
        }

        let mic = Recorder::open(RecorderConfig::speech())
            .map_err(|e| format!("Could not open the microphone: {e}"))?;
        mic.start(CollectingSink::new())
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
            #[cfg(target_os = "macos")]
            system,
        });
        drop(slot);
        Ok(self.status())
    }

    /// Stops recording and returns the History entry being transcribed.
    pub fn stop(&self, app: &AppHandle) -> Result<HistoryEntry, String> {
        let active = self.lock().take().ok_or("No meeting is being recorded")?;

        let mic = match active.mic.stop() {
            Ok(stopped) => stopped.sink.into_samples(),
            Err(e) => return Err(format!("Could not stop the microphone: {e}")),
        };
        let _ = active.mic.close();

        #[cfg(target_os = "macos")]
        let system = active.system.map(|capture| {
            let rate = capture.sample_rate;
            to_model_rate(&capture.stop(), rate)
        });
        #[cfg(not(target_os = "macos"))]
        let system: Option<Vec<f32>> = None;

        let mixed = mix(&mic, system.as_deref().unwrap_or(&[]));
        if (mixed.len() as f32 / MODEL_RATE as f32) < MIN_SECONDS {
            return Err("That recording was too short to keep".into());
        }

        let name = format!(
            "Meeting {}.wav",
            chrono::Local::now().format("%Y-%m-%d %H-%M")
        );
        let path = std::env::temp_dir().join(&name);
        save_wav_file(&path, &mixed).map_err(|e| e.to_string())?;
        let jobs = app.state::<Arc<TranscriptionJobManager>>();
        let entry = jobs
            .transcribe_file(path.clone())
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
        } else if let Err(e) = manager.start() {
            log::warn!("Could not start the meeting: {e}");
        }
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
    fn quiet_audio_is_left_alone() {
        assert_eq!(mix(&[0.1, 0.2], &[0.1, 0.1]), vec![0.2, 0.3]);
    }
}
