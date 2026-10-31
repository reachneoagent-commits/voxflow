//! "Focus on my voice": drops speech that is much quieter than the person
//! dictating, such as a conversation across the room that the microphone picks
//! up. It keeps the person's own recent speech level as a reference and lets a
//! frame through only if it's within a set margin of that level.

/// How hard to filter. Margins are in decibels below the speaker's level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusMargin {
    Gentle,
    Normal,
    Strong,
}

impl FocusMargin {
    fn db(self) -> f32 {
        match self {
            FocusMargin::Gentle => 18.0,
            FocusMargin::Normal => 12.0,
            FocusMargin::Strong => 8.0,
        }
    }
}

/// Frames to pass unfiltered while the speaker's level is first measured.
const WARMUP_FRAMES: u32 = 6;
/// Frames to keep passing after a loud one, so quiet syllables inside the
/// speaker's own words aren't clipped (about 0.4 s at 32 ms frames).
const HOLD_FRAMES: u32 = 12;
/// How fast the reference rises to a louder voice, and settles toward a quieter
/// one. Falling is slow so a short dip doesn't lower the bar for background talk.
const RISE: f32 = 0.2;
const FALL: f32 = 0.01;
/// Per dropped frame. If the speaker genuinely gets quieter, filtered speech
/// slowly lowers the bar so they aren't cut off for the rest of the recording.
const DROPPED_DECAY: f32 = 0.998;

pub struct FocusGate {
    threshold_ratio: f32,
    smoothed: f32,
    reference: f32,
    warmup: u32,
    hold: u32,
}

impl FocusGate {
    pub fn new(margin: FocusMargin) -> Self {
        Self {
            threshold_ratio: 10f32.powf(-margin.db() / 20.0),
            smoothed: 0.0,
            reference: 0.0,
            warmup: WARMUP_FRAMES,
            hold: 0,
        }
    }

    /// Whether this frame of speech (already judged to be speech by the VAD)
    /// should be kept.
    pub fn keep(&mut self, frame: &[f32]) -> bool {
        if frame.is_empty() {
            return true;
        }
        let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();
        self.smoothed = if self.smoothed == 0.0 {
            rms
        } else {
            0.5 * self.smoothed + 0.5 * rms
        };

        let loud_enough = self.smoothed >= self.reference * self.threshold_ratio;
        let keep = self.warmup > 0 || loud_enough || self.hold > 0;

        if self.warmup > 0 {
            self.warmup -= 1;
        }
        if loud_enough {
            self.hold = HOLD_FRAMES;
        } else if self.hold > 0 {
            self.hold -= 1;
        }

        if keep {
            let alpha = if self.smoothed > self.reference {
                RISE
            } else {
                FALL
            };
            self.reference += (self.smoothed - self.reference) * alpha;
        } else {
            self.reference *= DROPPED_DECAY;
        }
        keep
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(level: f32) -> Vec<f32> {
        vec![level; 512]
    }

    fn run(gate: &mut FocusGate, level: f32, frames: usize) -> usize {
        (0..frames).filter(|_| gate.keep(&frame(level))).count()
    }

    #[test]
    fn keeps_the_speaker_and_drops_much_quieter_background_talk() {
        let mut gate = FocusGate::new(FocusMargin::Normal);
        assert_eq!(run(&mut gate, 0.3, 40), 40, "the speaker is kept");
        // The hold and level smoothing let a few frames through, then a voice
        // ~20 dB quieter is dropped for good.
        let settling = run(&mut gate, 0.03, 20);
        assert!(settling <= HOLD_FRAMES as usize + 4, "kept {settling}");
        assert_eq!(
            run(&mut gate, 0.03, 40),
            0,
            "background talk still gets through"
        );
    }

    #[test]
    fn a_moderately_quieter_voice_from_the_speaker_still_passes() {
        let mut gate = FocusGate::new(FocusMargin::Normal);
        run(&mut gate, 0.3, 40);
        // About 7 dB down: within the 12 dB margin.
        assert_eq!(run(&mut gate, 0.13, 40), 40);
    }

    #[test]
    fn quiet_syllables_inside_a_word_are_held_through() {
        let mut gate = FocusGate::new(FocusMargin::Normal);
        run(&mut gate, 0.3, 40);
        assert_eq!(run(&mut gate, 0.01, 5), 5);
    }

    #[test]
    fn stronger_filtering_drops_more() {
        let mut gentle = FocusGate::new(FocusMargin::Gentle);
        let mut strong = FocusGate::new(FocusMargin::Strong);
        for gate in [&mut gentle, &mut strong] {
            run(gate, 0.3, 40);
        }
        assert!(run(&mut strong, 0.07, 60) < run(&mut gentle, 0.07, 60));
    }

    #[test]
    fn a_speaker_who_gets_much_quieter_is_not_cut_off_for_good() {
        let mut gate = FocusGate::new(FocusMargin::Normal);
        run(&mut gate, 0.3, 40);
        // A long stretch at ~25 dB lower: dropped at first, admitted eventually.
        let kept = run(&mut gate, 0.017, 2000);
        assert!(kept > 100, "only {kept} frames kept");
    }
}
