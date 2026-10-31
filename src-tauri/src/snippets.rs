//! Spoken-trigger text expansion ("my address" -> the saved address).
//!
//! Applied to the final text just before output, after any LLM post-processing,
//! so the saved text is inserted verbatim and never rewritten by the model.

use crate::settings::Snippet;
use regex::Regex;

pub const MAX_SNIPPETS: usize = 500;
pub const MAX_TRIGGER_CHARS: usize = 100;
pub const MAX_TEXT_CHARS: usize = 10_000;

/// Regex for one phrase: words in order, separated by whitespace (Whisper may
/// add a comma or full stop between them), matched case-insensitively on word
/// boundaries. With `swallow_trailing`, punctuation directly after the phrase is
/// dropped when it ends the utterance, so "calendar link." doesn't become
/// "<url>.".
pub(crate) fn phrase_pattern(phrase: &str, swallow_trailing: bool) -> Option<String> {
    let words: Vec<&str> = phrase.split_whitespace().collect();
    let first = words.first()?.chars().next()?;
    let last = words.last()?.chars().last()?;
    if !phrase.chars().any(char::is_alphanumeric) {
        return None;
    }
    let body = words
        .iter()
        .map(|word| regex::escape(word))
        .collect::<Vec<_>>()
        .join(r"[,.;:!?]?\s+");
    let start = if first.is_alphanumeric() || first == '_' {
        r"\b"
    } else {
        ""
    };
    let end = if last.is_alphanumeric() || last == '_' {
        r"\b"
    } else {
        ""
    };
    let tail = if swallow_trailing {
        r"(?:[.!?,;:]+\s*$)?"
    } else {
        ""
    };
    Some(format!(r"{start}{body}{end}{tail}"))
}

fn trigger_pattern(trigger: &str) -> Option<String> {
    phrase_pattern(trigger, true)
}

/// Replaces every phrase in `pairs` (phrase, replacement) found in `text`.
/// One pass over the original text: replacements are never re-scanned, so one
/// can't cascade into another. The longest phrase wins at a given spot.
pub(crate) fn expand_phrases(
    text: &str,
    pairs: &[(String, String)],
    swallow_trailing: bool,
) -> String {
    let mut active: Vec<(&String, &String, String)> = pairs
        .iter()
        .filter_map(|(phrase, replacement)| {
            phrase_pattern(phrase, swallow_trailing).map(|p| (phrase, replacement, p))
        })
        .collect();
    if active.is_empty() {
        return text.to_string();
    }
    active.sort_by_key(|(phrase, _, _)| std::cmp::Reverse(phrase.chars().count()));

    let pattern = active
        .iter()
        .enumerate()
        .map(|(i, (_, _, p))| format!("(?P<s{i}>{p})"))
        .collect::<Vec<_>>()
        .join("|");
    let Ok(regex) = Regex::new(&format!("(?i){pattern}")) else {
        return text.to_string();
    };

    regex
        .replace_all(text, |caps: &regex::Captures| {
            for (i, (_, replacement, _)) in active.iter().enumerate() {
                if caps.name(&format!("s{i}")).is_some() {
                    return (*replacement).clone();
                }
            }
            caps[0].to_string()
        })
        .into_owned()
}

/// Replaces every enabled snippet trigger in `text` with its saved text.
pub fn apply_snippets(text: &str, snippets: &[Snippet]) -> String {
    apply_snippets_with(text, snippets, Vec::new())
}

/// Like [`apply_snippets`], with more phrases and their replacements expanded in
/// the same pass (used to insert the clipboard).
pub fn apply_snippets_with(
    text: &str,
    snippets: &[Snippet],
    extra: Vec<(String, String)>,
) -> String {
    let mut pairs: Vec<(String, String)> = snippets
        .iter()
        .filter(|snippet| snippet.enabled)
        .map(|snippet| (snippet.trigger.clone(), snippet.text.clone()))
        .collect();
    pairs.extend(extra);
    expand_phrases(text, &pairs, true)
}

/// Whether `phrase` was spoken in `text`. Lets callers fetch something costly or
/// private, like the clipboard, only when it's actually asked for.
pub fn contains_phrase(text: &str, phrase: &str) -> bool {
    phrase_pattern(phrase, false)
        .and_then(|pattern| Regex::new(&format!("(?i){pattern}")).ok())
        .is_some_and(|regex| regex.is_match(text))
}

/// Rejects lists the UI should never produce, and trims triggers.
pub fn validate_snippets(snippets: Vec<Snippet>) -> Result<Vec<Snippet>, String> {
    if snippets.len() > MAX_SNIPPETS {
        return Err(format!("At most {MAX_SNIPPETS} snippets are supported"));
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(snippets.len());
    for mut snippet in snippets {
        snippet.trigger = snippet
            .trigger
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if trigger_pattern(&snippet.trigger).is_none() {
            return Err("A snippet trigger needs at least one letter or digit".into());
        }
        if snippet.trigger.chars().count() > MAX_TRIGGER_CHARS {
            return Err(format!(
                "Triggers are limited to {MAX_TRIGGER_CHARS} characters"
            ));
        }
        if snippet.text.chars().count() > MAX_TEXT_CHARS {
            return Err(format!(
                "Snippet text is limited to {MAX_TEXT_CHARS} characters"
            ));
        }
        if !seen.insert(snippet.trigger.to_lowercase()) {
            return Err(format!("Duplicate trigger: {}", snippet.trigger));
        }
        out.push(snippet);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snip(trigger: &str, text: &str) -> Snippet {
        Snippet {
            id: trigger.into(),
            trigger: trigger.into(),
            text: text.into(),
            enabled: true,
        }
    }

    #[test]
    fn expands_case_insensitively_inside_a_sentence() {
        let s = [snip("my address", "221B Baker Street")];
        assert_eq!(
            apply_snippets("Send it to My Address please", &s),
            "Send it to 221B Baker Street please"
        );
    }

    #[test]
    fn tolerates_punctuation_between_trigger_words() {
        let s = [snip("my address", "X")];
        assert_eq!(apply_snippets("my, address", &s), "X");
    }

    #[test]
    fn swallows_trailing_punctuation_only_at_end_of_text() {
        let s = [snip("calendar link", "https://cal.com/a")];
        assert_eq!(apply_snippets("Calendar link.", &s), "https://cal.com/a");
        assert_eq!(
            apply_snippets("Calendar link. Thanks", &s),
            "https://cal.com/a. Thanks"
        );
    }

    #[test]
    fn longest_trigger_wins() {
        let s = [snip("my address", "short"), snip("my address book", "long")];
        assert_eq!(apply_snippets("open my address book", &s), "open long");
    }

    #[test]
    fn does_not_match_inside_words() {
        let s = [snip("cat", "dog")];
        assert_eq!(apply_snippets("concatenate", &s), "concatenate");
    }

    #[test]
    fn expansion_is_not_rescanned_and_dollars_are_literal() {
        let s = [snip("a", "b a $1 ${x}")];
        assert_eq!(apply_snippets("a", &s), "b a $1 ${x}");
    }

    #[test]
    fn extra_phrase_expands_alongside_snippets() {
        let s = [snip("my address", "221B Baker Street")];
        let extra = vec![
            ("paste clipboard".to_string(), "https://x.com/a".to_string()),
            ("paste link".to_string(), "https://x.com/a".to_string()),
        ];
        assert_eq!(
            apply_snippets_with("see paste clipboard, then my address", &s, extra.clone()),
            "see https://x.com/a, then 221B Baker Street"
        );
        assert_eq!(
            apply_snippets_with(
                "check out this video paste link it's good",
                &s,
                extra.clone()
            ),
            "check out this video https://x.com/a it's good"
        );
        assert_eq!(
            apply_snippets_with("Paste clipboard.", &s, extra),
            "https://x.com/a"
        );
    }

    #[test]
    fn detects_a_spoken_phrase() {
        assert!(contains_phrase(
            "look, Paste clipboard please",
            "paste clipboard"
        ));
        assert!(contains_phrase("paste, clipboard", "paste clipboard"));
        assert!(!contains_phrase(
            "copy and paste the file",
            "paste clipboard"
        ));
    }

    #[test]
    fn skips_disabled_snippets() {
        let mut one = snip("hi", "hello");
        one.enabled = false;
        assert_eq!(apply_snippets("hi", &[one]), "hi");
    }

    #[test]
    fn validation_rejects_duplicates_and_empty_triggers() {
        assert!(validate_snippets(vec![snip("a", "1"), snip("A", "2")]).is_err());
        assert!(validate_snippets(vec![snip("  ", "1")]).is_err());
        assert!(
            validate_snippets(vec![snip("  my   address ", "1")]).unwrap()[0].trigger
                == "my address"
        );
    }
}
