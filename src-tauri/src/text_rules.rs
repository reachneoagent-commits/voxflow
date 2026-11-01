//! Plain-text rules applied to a finished transcription: dictionary
//! replacements, per-app writing styles, the "scratch that" command and word
//! counting. Everything here is local and deterministic (no model involved).

use crate::settings::{Replacement, WritingStyle};
use crate::snippets::expand_phrases;

pub const MAX_REPLACEMENTS: usize = 500;
pub const MAX_PHRASE_CHARS: usize = 100;

/// Swaps each enabled replacement's `from` phrase for its `to` text.
pub fn apply_replacements(text: &str, replacements: &[Replacement]) -> String {
    let pairs: Vec<(String, String)> = replacements
        .iter()
        .filter(|r| r.enabled)
        .map(|r| (r.from.clone(), r.to.clone()))
        .collect();
    expand_phrases(text, &pairs, false)
}

/// Rejects lists the UI should never produce, and tidies whitespace.
pub fn validate_replacements(replacements: Vec<Replacement>) -> Result<Vec<Replacement>, String> {
    if replacements.len() > MAX_REPLACEMENTS {
        return Err(format!(
            "At most {MAX_REPLACEMENTS} replacements are supported"
        ));
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(replacements.len());
    for mut r in replacements {
        r.from = r.from.split_whitespace().collect::<Vec<_>>().join(" ");
        r.to = r.to.trim().to_string();
        if !r.from.chars().any(char::is_alphanumeric) {
            return Err("A replacement needs at least one letter or digit to find".into());
        }
        if r.from.chars().count() > MAX_PHRASE_CHARS || r.to.chars().count() > MAX_PHRASE_CHARS {
            return Err(format!(
                "Replacements are limited to {MAX_PHRASE_CHARS} characters"
            ));
        }
        if !seen.insert(r.from.to_lowercase()) {
            return Err(format!("Duplicate replacement: {}", r.from));
        }
        out.push(r);
    }
    Ok(out)
}

fn is_sentence_end(c: char) -> bool {
    matches!(c, '.' | '!' | '?')
}

/// Applies `f` to the first letter of every sentence. A sentence starts at the
/// beginning, after `.`/`!`/`?` followed by whitespace, and after a newline.
/// `f` gets the first letter and the rest of its word.
fn map_sentence_starts(text: &str, f: impl Fn(char, &str) -> String) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut at_start = true;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if at_start && c.is_alphabetic() {
            let mut j = i + 1;
            while j < chars.len()
                && (chars[j].is_alphanumeric() || chars[j] == '\'' || chars[j] == '’')
            {
                j += 1;
            }
            let rest: String = chars[i + 1..j].iter().collect();
            out.push_str(&f(c, &rest));
            at_start = false;
            i = j;
            continue;
        }
        if c.is_alphanumeric() {
            at_start = false;
        } else if c == '\n' {
            at_start = true;
        } else if is_sentence_end(c) && chars.get(i + 1).is_none_or(|n| n.is_whitespace()) {
            at_start = true;
        }
        out.push(c);
        i += 1;
    }
    out
}

fn is_i_word(first: char, rest: &str) -> bool {
    first.eq_ignore_ascii_case(&'i')
        && (rest.is_empty()
            || matches!(
                rest.to_lowercase().as_str(),
                "'m" | "'ll" | "'ve" | "'d" | "’m" | "’ll" | "’ve" | "’d"
            ))
}

/// Capitalises sentence starts, a standalone "i" and adds a closing full stop.
fn formal(text: &str) -> String {
    let mut out = map_sentence_starts(text, |first, rest| {
        format!("{}{}", first.to_uppercase(), rest)
    });
    out = replace_standalone_i(&out);
    let trimmed = out.trim_end();
    match trimmed.chars().last() {
        Some(c) if c.is_alphanumeric() => format!("{trimmed}."),
        _ => out,
    }
}

fn replace_standalone_i(text: &str) -> String {
    let re = regex::Regex::new(r"\bi\b").expect("valid regex");
    re.replace_all(text, "I").into_owned()
}

/// Lowercases sentence starts (keeping "I", acronyms and names with inner
/// capitals) and drops the final full stop.
fn very_casual(text: &str) -> String {
    let lowered = map_sentence_starts(text, |first, rest| {
        let rest_is_plain = rest.chars().all(|c| !c.is_uppercase());
        if is_i_word(first, rest) || !rest_is_plain {
            format!("{first}{rest}")
        } else {
            format!("{}{}", first.to_lowercase(), rest)
        }
    });
    strip_final_period(&lowered)
}

fn strip_final_period(text: &str) -> String {
    let trimmed = text.trim_end();
    if let Some(stripped) = trimmed.strip_suffix('.') {
        // Keep ellipses.
        if !stripped.ends_with('.') {
            return stripped.to_string();
        }
    }
    text.to_string()
}

/// Leaves casing alone and drops the final full stop of a one-sentence line,
/// which suits commands, search boxes and code editors.
fn technical(text: &str) -> String {
    let trimmed = text.trim_end();
    let body = trimmed.strip_suffix('.').unwrap_or(trimmed);
    let has_inner_break = body.char_indices().any(|(i, c)| {
        is_sentence_end(c) && body[i + c.len_utf8()..].starts_with(char::is_whitespace)
    });
    if has_inner_break {
        text.to_string()
    } else {
        strip_final_period(text)
    }
}

pub fn apply_style(text: &str, style: WritingStyle) -> String {
    match style {
        WritingStyle::Casual => text.to_string(),
        WritingStyle::Formal => formal(text),
        WritingStyle::VeryCasual => very_casual(text),
        WritingStyle::Technical => technical(text),
    }
}

/// What a spoken "scratch that" does to this dictation.
#[derive(Debug, PartialEq, Eq)]
pub enum Backtrack {
    /// No correction was spoken: type the text as is.
    None,
    /// Only "scratch that": type nothing and undo the previous dictation.
    UndoPrevious,
    /// Everything said so far was scratched and nothing replaced it.
    Discard,
    /// A correction: type only what came after it.
    Text(String),
}

/// Words that may sit just before "scratch that" ("no, scratch that").
const LEAD_IN: &[&str] = &["no", "wait", "sorry", "oh", "um", "uh", "hmm", "ok", "okay"];
/// Words that introduce the replacement ("scratch that, I meant three").
const REPLACEMENT_MARKERS: &[&str] = &[
    "what i meant to say was",
    "what i meant to say is",
    "what i meant was",
    "what i meant is",
    "what i mean is",
    "let me say that again",
    "let me rephrase",
    "i meant to say",
    "i mean to say",
    "i should say",
    "i meant",
    "i mean",
    "or rather",
    "correction",
    "actually",
    "rather",
    "instead",
];

/// How a self-correction phrase said on its own (no "scratch that") has to be
/// set off before it counts.
#[derive(Clone, Copy)]
enum Need {
    /// The wording is explicit enough: only a pause before it.
    Explicit,
    /// "I meant" also starts ordinary sentences ("I meant to call"), so it must
    /// not lead into an infinitive or "that".
    NotInfinitive,
    /// Needs a pause on both sides.
    PausedAfter,
}

/// Correction phrases that work without "scratch that", longest first.
const STANDALONE: &[(&[&str], Need)] = &[
    (&["what", "i", "meant", "to", "say", "was"], Need::Explicit),
    (&["what", "i", "meant", "to", "say", "is"], Need::Explicit),
    (&["what", "i", "meant", "was"], Need::Explicit),
    (&["what", "i", "meant", "is"], Need::Explicit),
    (&["what", "i", "mean", "is"], Need::Explicit),
    (&["i", "meant", "to", "say"], Need::Explicit),
    (&["or", "rather"], Need::Explicit),
    (&["i", "meant"], Need::NotInfinitive),
    (&["correction"], Need::PausedAfter),
];
const PUNCTUATION: &[char] = &[',', '.', ';', ':', '!', '?'];

struct Token<'a> {
    start: usize,
    end: usize,
    word: &'a str,
}

fn tokens(text: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        let in_word = c.is_alphanumeric() || c == '\'' || c == '’';
        match (in_word, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push(Token {
                    start: s,
                    end: i,
                    word: &text[s..i],
                });
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(Token {
            start: s,
            end: text.len(),
            word: &text[s..],
        });
    }
    out
}

fn starts_with_marker(rest: &str) -> bool {
    let lowered = rest
        .trim_start_matches(|c: char| c.is_whitespace() || PUNCTUATION.contains(&c))
        .to_lowercase();
    REPLACEMENT_MARKERS.iter().any(|m| {
        lowered.strip_prefix(m).is_some_and(|after| {
            after.is_empty() || !after.starts_with(|c: char| c.is_alphanumeric())
        })
    })
}

/// Drops leading replacement markers and punctuation: ", I meant three" -> "three".
fn strip_replacement_lead(rest: &str) -> String {
    let mut text = rest.trim_start_matches(|c: char| c.is_whitespace() || PUNCTUATION.contains(&c));
    loop {
        let lowered = text.to_lowercase();
        let Some(marker) = REPLACEMENT_MARKERS.iter().find(|m| {
            lowered
                .strip_prefix(**m)
                .is_some_and(|a| a.is_empty() || !a.starts_with(|c: char| c.is_alphanumeric()))
        }) else {
            break;
        };
        text = text[marker.len()..]
            .trim_start_matches(|c: char| c.is_whitespace() || PUNCTUATION.contains(&c));
    }
    text.trim().to_string()
}

/// The most words one "scratch that" removes when there's no comma or full
/// stop to mark where the last phrase began.
const MAX_SCRATCH_WORDS: usize = 6;
/// Words removed when nothing says how much was meant: just the last few.
const BLIND_SCRATCH_WORDS: usize = 4;
/// Blind removal clears the phrase instead of leaving a fragment this short.
const MAX_FRAGMENT_WORDS: usize = 2;
/// How far back a replacement's first word is looked for when lining it up
/// with what it replaces ("send it to Mary" restating "send the report to John").
const ALIGN_WINDOW_WORDS: usize = 10;
/// Words a time, place or person usually follows; kept when only that part is replaced.
const PREPOSITIONS: &[&str] = &[
    "at", "on", "in", "to", "by", "for", "from", "with", "of", "around", "about", "before", "after",
];
/// Words that can't end a sentence; dropped when a removal leaves one hanging.
const DANGLING: &[&str] = &[
    "and", "or", "but", "so", "then", "to", "the", "a", "an", "of", "with", "for",
];

fn trim_pauses(text: &str) -> &str {
    text.trim_end_matches(|c: char| c.is_whitespace() || PUNCTUATION.contains(&c))
}

/// Whether the word right after a command also appears just before it, which
/// marks a correction even with no pause: "the other OS operating, scratch
/// that, operating systems".
fn echoes_before(toks: &[Token<'_>], command_index: usize, rest: &str) -> bool {
    tokens(rest).first().is_some_and(|next| {
        toks[..command_index]
            .iter()
            .rev()
            .take(MAX_SCRATCH_WORDS)
            .any(|t| t.word.eq_ignore_ascii_case(next.word))
    })
}

/// The first spoken correction command, as a byte range. Commands are
/// "scratch that", "undo that" and "never mind", which count when set off like
/// one: after a pause or a lead-in word ("no, scratch that") and followed by a
/// pause, the end, or the replacement ("I meant ..."). That keeps "scratch that
/// itch" as plain speech. "Actually" is much more common in ordinary speech, so
/// it only counts between pauses and when what follows repeats a word just said.
fn find_command(text: &str) -> Option<(usize, usize)> {
    let toks = tokens(text);
    let mut found: Option<(usize, usize)> = None;
    let mut keep_earliest = |range: (usize, usize)| {
        if found.is_none_or(|f| range.0 < f.0) {
            found = Some(range);
        }
    };

    for (i, pair) in toks.windows(2).enumerate() {
        let first = pair[0].word.to_lowercase();
        let second = pair[1].word.to_lowercase();
        let is_command = ((first == "scratch" || first == "undo") && second == "that")
            || (first == "never" && second == "mind");
        if !is_command {
            continue;
        }
        let (start, end) = (pair[0].start, pair[1].end);
        let prev = i.checked_sub(1).map(|p| &toks[p]);
        // Said at the start of a clause or sentence: "...Monday. Scratch that
        // Tuesday". That's a command even when nothing pauses after it.
        let starts_clause = prev.is_some_and(|prev| text[prev.end..start].contains(PUNCTUATION));
        let before_ok = match prev {
            None => true,
            Some(prev) => starts_clause || LEAD_IN.contains(&prev.word.to_lowercase().as_str()),
        };
        let rest = &text[end..];
        let after_pause = rest.trim().is_empty() || rest.trim_start().starts_with(PUNCTUATION);
        let marker = starts_with_marker(rest);
        let echo = echoes_before(&toks, i, rest);
        if starts_clause
            || (before_ok && (after_pause || marker))
            || (marker && !rest.trim().is_empty())
            || echo
        {
            keep_earliest((start, end));
            break;
        }
    }

    for (i, tok) in toks.iter().enumerate() {
        if i == 0 || !tok.word.eq_ignore_ascii_case("actually") {
            continue;
        }
        let rest = &text[tok.end..];
        let paused_before = text[toks[i - 1].end..tok.start].contains(PUNCTUATION);
        if paused_before
            && rest.trim_start().starts_with(PUNCTUATION)
            && echoes_before(&toks, i, rest)
        {
            keep_earliest((tok.start, tok.end));
            break;
        }
    }
    for (i, tok) in toks.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let paused_before = text[toks[i - 1].end..tok.start].contains(PUNCTUATION);
        if !paused_before {
            continue;
        }
        for (words, need) in STANDALONE {
            let matches = toks.len() >= i + words.len()
                && words
                    .iter()
                    .enumerate()
                    .all(|(k, w)| toks[i + k].word.eq_ignore_ascii_case(w));
            if !matches {
                continue;
            }
            let end = toks[i + words.len() - 1].end;
            let rest = &text[end..];
            let next = tokens(rest).first().map(|t| t.word.to_lowercase());
            let ok = !rest.trim().is_empty()
                && match need {
                    Need::Explicit => true,
                    Need::NotInfinitive => !matches!(next.as_deref(), Some("to" | "that")),
                    Need::PausedAfter => rest.trim_start().starts_with(PUNCTUATION),
                };
            if ok {
                keep_earliest((tok.start, end));
            }
            break;
        }
    }
    found
}

/// Removes the phrase said just before "scratch that" from `before`: back to
/// the previous comma or full stop. Lead-in words right before the command
/// ("no,") go with it.
///
/// How many words go depends on what follows:
/// - nothing says how much: the last few words (`BLIND_SCRATCH_WORDS`), or the
///   whole phrase if that would leave only a fragment;
/// - the replacement restates the end of the phrase ("Monday in an email" then
///   "Tuesday in an email"): as many words as the replacement has;
/// - the replacement starts with a word also said just before ("to John" then
///   "to Mary"): from that word on;
/// - otherwise: about as many words as the replacement has, at most three.
fn drop_last_phrase(before: &str, replacement: Option<&str>) -> String {
    let mut text = trim_pauses(before).to_string();
    while let Some(last) = tokens(&text).last() {
        if !LEAD_IN.contains(&last.word.to_lowercase().as_str()) {
            break;
        }
        text.truncate(last.start);
        text = trim_pauses(&text).to_string();
    }
    let phrase_start = text.rfind(PUNCTUATION).map_or(0, |i| i + 1);
    let mut kept = text[..phrase_start].to_string();
    let phrase = &text[phrase_start..];
    let words = tokens(phrase);
    let window_start = words.len().saturating_sub(ALIGN_WINDOW_WORDS);

    let remove = match replacement {
        None => {
            let remove = BLIND_SCRATCH_WORDS.min(words.len());
            if words.len() - remove > 0 && words.len() - remove <= MAX_FRAGMENT_WORDS {
                words.len()
            } else {
                remove
            }
        }
        Some(replacement) => {
            let rw = tokens(replacement);
            let lowered = |t: &Token<'_>| t.word.to_lowercase();
            let shared_tail = words
                .iter()
                .rev()
                .zip(rw.iter().rev())
                .take_while(|(a, b)| lowered(a) == lowered(b))
                .count();
            let aligned = rw.first().and_then(|first| {
                (window_start..words.len())
                    .rev()
                    .find(|&i| lowered(&words[i]) == lowered(first))
            });
            if shared_tail >= 2 {
                rw.len().min(words.len())
            } else if rw.len() >= words.len() && words.len() <= ALIGN_WINDOW_WORDS {
                // As long as what it replaces: the whole phrase was restated.
                words.len()
            } else if let Some(i) = aligned {
                words.len() - i
            } else {
                let guess = if rw.len() > 3 { 1 } else { rw.len().max(1) };
                let mut remove = guess.min(words.len());
                // "at noon" replaced by "one o'clock": keep the "at".
                let first_removed = &words[words.len() - remove];
                if remove >= 2
                    && PREPOSITIONS.contains(&lowered(first_removed).as_str())
                    && !rw
                        .first()
                        .is_some_and(|f| PREPOSITIONS.contains(&lowered(f).as_str()))
                {
                    remove -= 1;
                }
                remove
            }
        }
    };
    let cut = words.len() - remove;
    if cut > 0 {
        kept.push_str(&phrase[..words[cut].start.min(phrase.len())]);
    }
    let mut kept = kept.trim_end().to_string();
    if replacement.is_none() {
        // Don't leave "...go home and" hanging.
        while let Some(last) = tokens(&kept).last() {
            if last.end != kept.len() || !DANGLING.contains(&last.word.to_lowercase().as_str()) {
                break;
            }
            kept.truncate(last.start);
            kept = kept.trim_end().to_string();
        }
    }
    kept
}

/// "Meet at 2 o'clock tomorrow, scratch that, 3 o'clock": a number or time
/// replaces the last number said, in place, rather than the end of the phrase.
/// Returns the corrected text (including whatever follows the replacement).
fn swap_number(before: &str, after: &str) -> Option<String> {
    let rep = replacement_part(after)?;
    let rw = tokens(rep);
    if rw.is_empty() || rw.len() > 3 || !rw[0].word.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let mut text = trim_pauses(before).to_string();
    while let Some(last) = tokens(&text).last() {
        if !LEAD_IN.contains(&last.word.to_lowercase().as_str()) {
            break;
        }
        text.truncate(last.start);
        text = trim_pauses(&text).to_string();
    }
    let phrase_start = text.rfind(PUNCTUATION).map_or(0, |i| i + 1);
    let phrase = &text[phrase_start..];
    let words = tokens(phrase);
    let at = words
        .iter()
        .rposition(|t| t.word.starts_with(|c: char| c.is_ascii_digit()))?;
    let last_replaced = (at + rw.len() - 1).min(words.len() - 1);
    let prefix = format!("{}{}", &text[..phrase_start], &phrase[..words[at].start]);
    let suffix = phrase[words[last_replaced].end..].trim_end();
    let replacement = rep.trim();
    let remainder = after[rep.len().min(after.len())..].trim_start();
    let joined = format!("{}{}{}", prefix.trim_end(), " ", replacement);
    let joined = format!("{joined}{suffix}");
    Some(if remainder.is_empty() {
        joined
    } else if remainder.starts_with(PUNCTUATION) {
        format!("{joined}{remainder}")
    } else {
        format!("{joined} {remainder}")
    })
}

/// The replacement's text up to its first pause, if there is any.
fn replacement_part(after: &str) -> Option<&str> {
    let part = after.split(PUNCTUATION).next().unwrap_or("");
    (!tokens(part).is_empty()).then_some(part)
}

fn has_words(text: &str) -> bool {
    tokens(text)
        .iter()
        .any(|t| !LEAD_IN.contains(&t.word.to_lowercase().as_str()))
}

/// Works out what a spoken "scratch that" does to this dictation: it removes
/// just the phrase said right before it, and anything after it is kept.
pub fn resolve_backtrack(text: &str) -> Backtrack {
    let mut current = text.to_string();
    let mut changed = false;
    for _ in 0..5 {
        let Some((start, end)) = find_command(&current) else {
            break;
        };
        let before = &current[..start];
        let after = strip_replacement_lead(&current[end..]);
        if !changed && !has_words(before) && after.is_empty() {
            return Backtrack::UndoPrevious;
        }
        // A command that is a sentence of its own ("... Monday. Scratch that.
        // Goodbye.") is followed by new dictation, not a replacement, unless the
        // speaker says so ("I meant ...").
        let rest = &current[end..];
        let standalone = rest.trim_start().starts_with(['.', '!', '?']);
        let replacement = if standalone && !starts_with_marker(rest) {
            None
        } else {
            replacement_part(&after)
        };
        if replacement.is_some() {
            if let Some(swapped) = swap_number(before, &after) {
                current = swapped;
                changed = true;
                continue;
            }
        }
        let kept = drop_last_phrase(before, replacement);
        current = match (kept.is_empty(), after.is_empty()) {
            (true, _) => after,
            (false, true) => kept.trim_end_matches([',', ';', ':']).to_string(),
            (false, false) => format!("{kept} {after}"),
        };
        changed = true;
    }
    if !changed {
        Backtrack::None
    } else if current.trim().is_empty() {
        Backtrack::Discard
    } else {
        Backtrack::Text(current.trim().to_string())
    }
}

/// What the live preview should show: the same corrections the final text gets,
/// so a scratched phrase disappears as it's recognised. Returns the (committed,
/// tentative) pair, keeping as much of the committed prefix as survived.
pub fn preview_with_corrections(committed: &str, tentative: &str) -> (String, String) {
    let full = format!("{committed}{tentative}");
    let lowered = full.to_lowercase();
    if ![
        "scratch",
        "undo",
        "never",
        "actually",
        "meant",
        "rather",
        "correction",
    ]
    .iter()
    .any(|word| lowered.contains(word))
    {
        return (committed.to_string(), tentative.to_string());
    }
    // A command that is the very last thing heard, with nothing after it yet,
    // could still become a one-word swap or a whole-sentence removal. Until more
    // arrives, just hide the command words, so the preview doesn't remove a
    // sentence and then bring most of it back.
    let shown = match find_command(&full) {
        Some((start, end)) if full[end..].trim().is_empty() => full[..start].trim_end().to_string(),
        _ => match resolve_backtrack(&full) {
            Backtrack::None => return (committed.to_string(), tentative.to_string()),
            Backtrack::Text(text) => text,
            Backtrack::UndoPrevious | Backtrack::Discard => String::new(),
        },
    };
    let common: usize = shown
        .chars()
        .zip(committed.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum();
    (shown[..common].to_string(), shown[common..].to_string())
}

pub fn count_words(text: &str) -> u32 {
    text.split_whitespace().count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rep(from: &str, to: &str) -> Replacement {
        Replacement {
            id: from.into(),
            from: from.into(),
            to: to.into(),
            enabled: true,
        }
    }

    #[test]
    fn replacements_keep_surrounding_punctuation() {
        let r = [rep("gonna", "going to")];
        assert_eq!(apply_replacements("I'm Gonna go.", &r), "I'm going to go.");
    }

    #[test]
    fn replacements_validate() {
        assert!(validate_replacements(vec![rep("a", "b"), rep("A", "c")]).is_err());
        assert!(validate_replacements(vec![rep("  ", "b")]).is_err());
        assert_eq!(
            validate_replacements(vec![rep(" my  word ", " x ")]).unwrap()[0].from,
            "my word"
        );
    }

    #[test]
    fn formal_capitalises_and_closes() {
        assert_eq!(
            apply_style("hello there. how are you", WritingStyle::Formal),
            "Hello there. How are you."
        );
        assert_eq!(
            apply_style("i think i can", WritingStyle::Formal),
            "I think I can."
        );
        assert_eq!(apply_style("Really?", WritingStyle::Formal), "Really?");
    }

    #[test]
    fn formal_ignores_decimals() {
        assert_eq!(
            apply_style("it costs 3.5 dollars", WritingStyle::Formal),
            "It costs 3.5 dollars."
        );
    }

    #[test]
    fn very_casual_lowercases_but_keeps_i_and_acronyms() {
        assert_eq!(
            apply_style("Hello there. I'm on NASA time.", WritingStyle::VeryCasual),
            "hello there. I'm on NASA time"
        );
        assert_eq!(apply_style("Sure!", WritingStyle::VeryCasual), "sure!");
    }

    #[test]
    fn technical_keeps_case_and_drops_single_period() {
        assert_eq!(
            apply_style("Run npm install.", WritingStyle::Technical),
            "Run npm install"
        );
        assert_eq!(
            apply_style("First do this. Then that.", WritingStyle::Technical),
            "First do this. Then that."
        );
    }

    #[test]
    fn casual_is_unchanged() {
        assert_eq!(
            apply_style("Hey. What's up.", WritingStyle::Casual),
            "Hey. What's up."
        );
    }

    #[test]
    fn scratch_that_alone_undoes_the_previous_dictation() {
        assert_eq!(resolve_backtrack("Scratch that."), Backtrack::UndoPrevious);
        assert_eq!(resolve_backtrack(" undo that! "), Backtrack::UndoPrevious);
        assert_eq!(
            resolve_backtrack("No, scratch that."),
            Backtrack::UndoPrevious
        );
    }

    #[test]
    fn scratch_that_removes_only_the_last_phrase() {
        assert_eq!(
            resolve_backtrack("I'll go to the store, and buy milk, scratch that, buy eggs"),
            Backtrack::Text("I'll go to the store, and buy eggs".into())
        );
        assert_eq!(
            resolve_backtrack("Hello there. Meet at noon. Scratch that. Meet at one."),
            Backtrack::Text("Hello there. Meet at one.".into())
        );
        assert_eq!(
            resolve_backtrack("Thanks for the update, I'll send it Monday, scratch that"),
            Backtrack::Text("Thanks for the update".into())
        );
    }

    #[test]
    fn a_correction_replaces_the_last_phrase() {
        assert_eq!(
            resolve_backtrack("No, scratch that, I meant three."),
            Backtrack::Text("three.".into())
        );
        assert_eq!(
            resolve_backtrack("Send it Monday, scratch that, Tuesday"),
            Backtrack::Text("Send it Tuesday".into())
        );
        assert_eq!(
            resolve_backtrack("Great. Scratch that, I meant awesome"),
            Backtrack::Text("awesome".into())
        );
        assert_eq!(
            resolve_backtrack("Meet at noon scratch that I meant one"),
            Backtrack::Text("Meet at one".into())
        );
        assert_eq!(
            resolve_backtrack("Please send the report to John, no, scratch that, to Mary"),
            Backtrack::Text("Please send the report to Mary".into())
        );
    }

    #[test]
    fn a_replacement_is_lined_up_with_what_it_replaces() {
        assert_eq!(
            resolve_backtrack("Send the email to John, scratch that, to Sarah Johnson"),
            Backtrack::Text("Send the email to Sarah Johnson".into())
        );
        assert_eq!(
            resolve_backtrack("Open the file in the editor, scratch that, the terminal"),
            Backtrack::Text("Open the file in the terminal".into())
        );
        assert_eq!(
            resolve_backtrack("Meet at noon, scratch that, I meant at one"),
            Backtrack::Text("Meet at one".into())
        );
    }

    #[test]
    fn ordinary_words_before_the_command_are_not_swallowed() {
        assert_eq!(
            resolve_backtrack("That was well done, so, scratch that, great job"),
            Backtrack::Text("That was well done, great job".into())
        );
    }

    #[test]
    fn repeating_a_word_after_scratch_that_corrects_without_a_pause() {
        assert_eq!(
            resolve_backtrack(
                "why can we not build for the other OS operating scratch that operating systems and how"
            ),
            Backtrack::Text("why can we not build for the other OS operating systems and how".into())
        );
        assert_eq!(
            resolve_backtrack("meet at noon scratch that at one"),
            Backtrack::Text("meet at one".into())
        );
        assert_eq!(
            resolve_backtrack("I need to scratch that itch"),
            Backtrack::None
        );
    }

    #[test]
    fn never_mind_works_like_scratch_that() {
        assert_eq!(
            resolve_backtrack("Send it Monday, never mind, Tuesday"),
            Backtrack::Text("Send it Tuesday".into())
        );
        assert_eq!(resolve_backtrack("Never mind."), Backtrack::UndoPrevious);
        assert_eq!(resolve_backtrack("never mind the cost"), Backtrack::None);
    }

    #[test]
    fn actually_only_counts_when_it_clearly_repeats_a_word() {
        assert_eq!(
            resolve_backtrack("Meet at two, actually, at three"),
            Backtrack::Text("Meet at three".into())
        );
        assert_eq!(resolve_backtrack("I actually liked it"), Backtrack::None);
        assert_eq!(
            resolve_backtrack("It was good, actually, it was great"),
            Backtrack::Text("it was great".into())
        );
        assert_eq!(resolve_backtrack("Actually, I liked it"), Backtrack::None);
        assert_eq!(
            resolve_backtrack("It is fine, actually, thanks"),
            Backtrack::None
        );
    }

    #[test]
    fn the_live_preview_applies_corrections_and_keeps_the_committed_prefix() {
        assert_eq!(
            preview_with_corrections("Send it Monday, scratch that,", " Tuesday"),
            ("Send it ".to_string(), "Tuesday".to_string())
        );
        assert_eq!(
            preview_with_corrections("Hello there", " how are"),
            ("Hello there".to_string(), " how are".to_string())
        );
        assert_eq!(
            preview_with_corrections("Meet at noon.", " Scratch that."),
            (String::new(), String::new())
        );
    }

    #[test]
    fn a_correction_said_as_a_new_sentence_swaps_one_word() {
        assert_eq!(
            resolve_backtrack(
                "I'm gonna create a list and I'm gonna send it over in an email Monday. Scratch that Tuesday and then I want to go ahead"
            ),
            Backtrack::Text(
                "I'm gonna create a list and I'm gonna send it over in an email Tuesday and then I want to go ahead"
                    .into()
            )
        );
    }

    #[test]
    fn never_mind_as_its_own_sentence_removes_the_whole_previous_sentence() {
        assert_eq!(
            resolve_backtrack(
                "I want to send it Monday in an email. Never mind. I meant to say Tuesday in an email. I'm trying to go"
            ),
            Backtrack::Text("I want to send it Tuesday in an email. I'm trying to go".into())
        );
        assert_eq!(
            resolve_backtrack("Hello there. I like it a lot. Scratch that. Goodbye."),
            Backtrack::Text("Hello there. Goodbye.".into())
        );
    }

    #[test]
    fn the_preview_only_hides_a_trailing_command_until_more_is_heard() {
        assert_eq!(
            preview_with_corrections("Send it Monday in an email. Scratch that", ""),
            ("Send it Monday in an email.".to_string(), String::new())
        );
        assert_eq!(
            preview_with_corrections("Send it in an email Monday. Scratch that Tuesday", ""),
            ("Send it in an email ".to_string(), "Tuesday".to_string())
        );
    }

    #[test]
    fn real_dictations_heard_by_the_speech_model() {
        let cases = [
            (
                "Please send the report to John by Friday afternoon. Scratch that, send it to Mary instead.",
                "Please send it to Mary instead.",
            ),
            (
                "We should meet at 2 o'clock tomorrow in the main conference room. Scratch that, 3 o'clock.",
                "We should meet at 3 o'clock tomorrow in the main conference room.",
            ),
            (
                "The weather is great today, and I plan to walk the dog before lunch. Scratch that, I plan to ride my bike.",
                "The weather is great today, and I plan to ride my bike.",
            ),
            (
                "Hello everyone, welcome to the meeting, let's get started with the agenda for today, scratch that, let's begin with introductions.",
                "Hello everyone, welcome to the meeting, let's begin with introductions.",
            ),
            (
                "I'm going to the store to buy some milk and eggs, then I'll go home and make dinner for everyone. Scratch that",
                "I'm going to the store to buy some milk and eggs, then I'll go home",
            ),
            (
                "Call me at 5, scratch that, at 6:30.",
                "Call me at 6:30.",
            ),
        ];
        for (heard, typed) in cases {
            assert_eq!(
                resolve_backtrack(heard),
                Backtrack::Text(typed.into()),
                "{heard}"
            );
        }
    }

    #[test]
    fn a_long_dictation_loses_only_the_last_few_words() {
        let heard = "Good morning team, I wanted to give you a quick update, we finished the first phase last week. The next step is to review the numbers with finance on Thursday. I also want everyone to send me their notes by Friday, scratch that.";
        let Backtrack::Text(typed) = resolve_backtrack(heard) else {
            panic!("expected text")
        };
        assert!(typed.starts_with("Good morning team, I wanted"));
        assert!(typed.contains("review the numbers with finance on Thursday."));
        assert!(
            typed.ends_with("I also want everyone to send me"),
            "{typed}"
        );
    }

    #[test]
    fn i_meant_and_similar_phrases_correct_without_scratch_that() {
        let cases = [
            ("Send it Monday, I meant to say Tuesday", "Send it Tuesday"),
            ("Send it Monday, I meant Tuesday", "Send it Tuesday"),
            ("Meet at two, what I meant was at three", "Meet at three"),
            ("Call John, or rather, call Mary", "call Mary"),
            (
                "It costs ten dollars. Correction, twelve dollars.",
                "It costs twelve dollars.",
            ),
        ];
        for (heard, typed) in cases {
            assert_eq!(
                resolve_backtrack(heard),
                Backtrack::Text(typed.into()),
                "{heard}"
            );
        }
        // Ordinary uses of the same words are left alone.
        for plain in [
            "I meant to call you yesterday",
            "Sorry. I meant to call you.",
            "I meant that sincerely",
            "The correction was small",
        ] {
            assert_eq!(resolve_backtrack(plain), Backtrack::None, "{plain}");
        }
    }

    #[test]
    fn scratch_that_with_longer_i_meant_phrases() {
        assert_eq!(
            resolve_backtrack("Send it Monday, scratch that, what I meant to say was Tuesday"),
            Backtrack::Text("Send it Tuesday".into())
        );
        assert_eq!(
            resolve_backtrack("Send it Monday, scratch that, I should say Tuesday"),
            Backtrack::Text("Send it Tuesday".into())
        );
    }

    #[test]
    fn restating_the_whole_phrase_and_keeping_prepositions() {
        assert_eq!(
            resolve_backtrack(
                "I like the blue one, scratch that, I meant to say the red one is better."
            ),
            Backtrack::Text("the red one is better.".into())
        );
        assert_eq!(
            resolve_backtrack("We will have lunch at noon, sorry, I meant one o'clock."),
            Backtrack::Text("We will have lunch at one o'clock.".into())
        );
    }

    #[test]
    fn with_no_pause_it_removes_at_most_a_few_words() {
        assert_eq!(
            resolve_backtrack("I will go to the store and buy some milk, scratch that"),
            Backtrack::Text("I will go to the store".into())
        );
    }

    #[test]
    fn scratching_the_only_phrase_discards_the_dictation() {
        assert_eq!(
            resolve_backtrack("Meet at noon. Scratch that."),
            Backtrack::Discard
        );
    }

    #[test]
    fn ordinary_speech_is_left_alone() {
        assert_eq!(
            resolve_backtrack("Please scratch that itch"),
            Backtrack::None
        );
        assert_eq!(
            resolve_backtrack("I want to scratch that."),
            Backtrack::None
        );
        assert_eq!(resolve_backtrack("Hello there"), Backtrack::None);
    }

    #[test]
    fn each_scratch_that_removes_one_more_phrase() {
        assert_eq!(
            resolve_backtrack("One, two, scratch that, three, scratch that, four"),
            Backtrack::Text("One, four".into())
        );
    }

    #[test]
    fn word_count() {
        assert_eq!(count_words("one two  three"), 3);
        assert_eq!(count_words(""), 0);
    }
}
