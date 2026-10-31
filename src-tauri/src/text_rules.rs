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
const REPLACEMENT_MARKERS: &[&str] = &["i meant", "i mean", "actually", "rather", "instead"];
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

fn trim_pauses(text: &str) -> &str {
    text.trim_end_matches(|c: char| c.is_whitespace() || PUNCTUATION.contains(&c))
}

/// The first spoken "scratch that" (or "undo that") that is set off like a
/// command: after a pause or a lead-in word ("no, scratch that") and followed
/// by a pause, the end, or the replacement ("I meant ..."). That keeps
/// "scratch that itch" as plain speech. Returns the phrase's byte range.
fn find_command(text: &str) -> Option<(usize, usize)> {
    let toks = tokens(text);
    for (i, pair) in toks.windows(2).enumerate() {
        let first = pair[0].word.to_lowercase();
        let second = pair[1].word.to_lowercase();
        if !((first == "scratch" || first == "undo") && second == "that") {
            continue;
        }
        let (start, end) = (pair[0].start, pair[1].end);
        let before_ok = match i.checked_sub(1).map(|p| &toks[p]) {
            None => true,
            Some(prev) => {
                text[prev.end..start].contains(PUNCTUATION)
                    || LEAD_IN.contains(&prev.word.to_lowercase().as_str())
            }
        };
        let rest = &text[end..];
        let after_pause = rest.trim().is_empty() || rest.trim_start().starts_with(PUNCTUATION);
        let marker = starts_with_marker(rest);
        // Saying the same word again right after is a correction even with no
        // pause: "the other OS operating, scratch that, operating systems".
        let echo = tokens(rest).first().is_some_and(|next| {
            toks[..i]
                .iter()
                .rev()
                .take(MAX_SCRATCH_WORDS)
                .any(|t| t.word.eq_ignore_ascii_case(next.word))
        });
        if (before_ok && (after_pause || marker)) || (marker && !rest.trim().is_empty()) || echo {
            return Some((start, end));
        }
    }
    None
}

/// Removes the phrase said just before "scratch that" from `before`: back to
/// the previous comma or full stop, and at most `MAX_SCRATCH_WORDS` words. Lead-in
/// words right before the command ("no,") go with it.
///
/// When a replacement follows, it's lined up with what it replaces: if its first
/// word also appears in the last few words, removal starts at that word
/// ("to John, scratch that, to Mary" swaps "to John"); otherwise about as many
/// words are removed as the replacement has, at most three.
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
    let window_start = words.len().saturating_sub(MAX_SCRATCH_WORDS);

    let cut = match replacement {
        None => window_start,
        Some(replacement) => {
            let replacement_words = tokens(replacement);
            let first = replacement_words.first().map(|t| t.word.to_lowercase());
            let aligned = first.and_then(|first| {
                (window_start..words.len())
                    .rev()
                    .find(|&i| words[i].word.to_lowercase() == first)
            });
            aligned.unwrap_or_else(|| {
                let n = replacement_words.len().clamp(1, 3).min(words.len());
                words.len() - n
            })
        }
    };
    if cut > 0 {
        kept.push_str(&phrase[..words[cut].start.min(phrase.len())]);
    }
    kept.trim_end().to_string()
}

/// The replacement's words up to its first pause, if there are any.
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
        let kept = drop_last_phrase(before, replacement_part(&after));
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
    fn with_no_pause_it_removes_at_most_a_few_words() {
        assert_eq!(
            resolve_backtrack("I will go to the store and buy some milk, scratch that"),
            Backtrack::Text("I will go to".into())
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
