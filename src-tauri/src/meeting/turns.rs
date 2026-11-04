//! Splits a meeting's two tracks into turns: "You" (the microphone) and
//! "Others" (everything the Mac played), each a stretch of speech between
//! pauses, so every turn can be transcribed and labelled on its own.

/// Samples per analysis frame (30 ms at 16 kHz).
const FRAME: usize = 480;
/// A pause shorter than this stays inside one turn (0.9 s).
const MERGE_GAP_FRAMES: usize = 30;
/// Anything shorter than this is a cough or a click, not speech (0.45 s).
const MIN_FRAMES: usize = 15;
/// Audio kept around a turn so a word is never clipped (0.15 s).
const PAD_FRAMES: usize = 5;
/// Quietest sound that can count as speech.
const FLOOR: f32 = 0.005;
/// Frames around the call's speech where the microphone is ignored, since the
/// speakers' sound arrives a moment late and rings on (0.12 s).
const BLEED_MARGIN_FRAMES: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speaker {
    You,
    Others,
}

/// One stretch of speech, with its own audio.
pub struct Turn {
    pub speaker: Speaker,
    /// Seconds into the meeting.
    pub start: f32,
    pub samples: Vec<f32>,
}

fn frame_levels(samples: &[f32]) -> Vec<f32> {
    samples
        .chunks(FRAME)
        .map(|frame| (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt())
        .collect()
}

/// Which frames carry speech: louder than a share of the track's loud parts.
fn active_frames(levels: &[f32]) -> Vec<bool> {
    if levels.is_empty() {
        return Vec::new();
    }
    let mut sorted = levels.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let loud = sorted[(sorted.len() * 9 / 10).min(sorted.len() - 1)];
    let threshold = FLOOR.max(0.15 * loud);
    levels.iter().map(|level| *level >= threshold).collect()
}

/// Frame ranges `(start, end)` of speech: nearby active frames join into one,
/// and a stretch too short to be speech is dropped.
fn spans(active: &[bool]) -> Vec<(usize, usize)> {
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for (i, on) in active.iter().enumerate() {
        if !on {
            continue;
        }
        match spans.last_mut() {
            Some((_, end)) if i - *end <= MERGE_GAP_FRAMES => *end = i + 1,
            _ => spans.push((i, i + 1)),
        }
    }
    spans.retain(|(start, end)| end - start >= MIN_FRAMES);
    spans
}

/// `active` with `margin` frames added around every active stretch.
fn widen(active: &[bool], margin: usize) -> Vec<bool> {
    (0..active.len())
        .map(|i| {
            let from = i.saturating_sub(margin);
            let to = (i + margin + 1).min(active.len());
            active[from..to].iter().any(|on| *on)
        })
        .collect()
}

fn cut(samples: &[f32], speaker: Speaker, (start, end): (usize, usize)) -> Turn {
    let from = start.saturating_sub(PAD_FRAMES) * FRAME;
    let to = ((end + PAD_FRAMES) * FRAME).min(samples.len());
    Turn {
        speaker,
        start: from as f32 / 16_000.0,
        samples: samples[from.min(to)..to].to_vec(),
    }
}

/// The meeting's turns in the order they were spoken.
pub fn split(mic: &[f32], system: &[f32]) -> Vec<Turn> {
    let system_active = active_frames(&frame_levels(system));
    let system_spans = spans(&system_active);

    // Without headphones the microphone hears the call too, so a microphone
    // frame only counts as you while the call is quiet.
    let call_heard = widen(&system_active, BLEED_MARGIN_FRAMES);
    let mic_active: Vec<bool> = active_frames(&frame_levels(mic))
        .into_iter()
        .enumerate()
        .map(|(i, on)| on && !call_heard.get(i).copied().unwrap_or(false))
        .collect();

    let mut turns: Vec<Turn> = system_spans
        .iter()
        .map(|span| cut(system, Speaker::Others, *span))
        .collect();
    turns.extend(
        spans(&mic_active)
            .iter()
            .map(|span| cut(mic, Speaker::You, *span)),
    );
    turns.sort_by(|a, b| a.start.total_cmp(&b.start));
    turns
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `seconds` of a tone at `level`, then silence to fill `total` seconds.
    fn track(parts: &[(f32, f32, f32)], total: f32) -> Vec<f32> {
        let mut out = vec![0.0; (total * 16_000.0) as usize];
        for (from, to, level) in parts {
            for i in (*from * 16_000.0) as usize..((*to * 16_000.0) as usize).min(out.len()) {
                out[i] = level * (i as f32 * 0.3).sin();
            }
        }
        out
    }

    #[test]
    fn turns_come_out_in_speaking_order_with_the_right_label() {
        let mic = track(&[(1.0, 3.0, 0.2), (9.0, 11.0, 0.2)], 12.0);
        let system = track(&[(4.0, 7.0, 0.2)], 12.0);
        let turns = split(&mic, &system);
        let who: Vec<Speaker> = turns.iter().map(|t| t.speaker).collect();
        assert_eq!(who, vec![Speaker::You, Speaker::Others, Speaker::You]);
        assert!(turns[0].start < turns[1].start && turns[1].start < turns[2].start);
    }

    #[test]
    fn a_short_pause_keeps_one_turn_and_a_long_one_splits_it() {
        let system = track(&[(1.0, 2.0, 0.2), (2.4, 3.0, 0.2), (6.0, 7.0, 0.2)], 8.0);
        let turns = split(&[0.0; 16_000 * 8], &system);
        assert_eq!(turns.len(), 2);
    }

    #[test]
    fn the_call_leaking_into_the_microphone_is_not_your_turn() {
        let system = track(&[(2.0, 6.0, 0.3)], 8.0);
        let mic = track(&[(2.0, 6.0, 0.05)], 8.0);
        let turns = split(&mic, &system);
        assert!(turns.iter().all(|t| t.speaker == Speaker::Others));
    }

    #[test]
    fn your_words_after_the_call_leaks_in_are_still_your_turn() {
        // The call speaks 2-5 s and leaks into the microphone; you speak 6-9 s.
        let system = track(&[(2.0, 5.0, 0.3)], 12.0);
        let mic = track(&[(2.0, 5.0, 0.05), (6.0, 9.0, 0.2)], 12.0);
        let turns = split(&mic, &system);
        let you: Vec<&Turn> = turns.iter().filter(|t| t.speaker == Speaker::You).collect();
        assert_eq!(you.len(), 1);
        assert!(you[0].start > 5.0);
    }

    #[test]
    fn silence_and_clicks_make_no_turns() {
        let mic = track(&[(1.0, 1.1, 0.3)], 4.0);
        assert!(split(&mic, &[0.0; 64_000]).is_empty());
    }
}
