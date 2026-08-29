//! The yak-reference model: how one yak refers to another in free text, and how
//! such a reference is detected and resolved. This is the single source of truth
//! for every consumer — the TUI (highlight/follow), the CLI (`yaks refs`), and
//! the rename tools — so they all agree on what counts as a reference.
//!
//! # Grammar
//!
//! - **Canonical form** — `<prefix>-<tail>`, e.g. `yak-0af1`. Detected as a
//!   maximal run of `[a-z0-9.-]` (see [`is_ref_char`]).
//! - **Wiki alias** — `[[yak-0af1]]`. The brackets are cosmetic; callers
//!   normalize them away with [`strip_wikilinks`] before scanning, so the
//!   rendered text shows a bare reference and offsets line up with the display.
//! - **Reserved (not yet implemented, see yaks-15f7)** — a herd-qualified form
//!   `<herd>:<prefix>-<tail>` for cross-herd references. `:` is deliberately not
//!   a ref char, so today `other:yak-0af1` scans as the local id `yak-0af1`;
//!   when federation lands, the qualifier is parsed here rather than at a caller.
//!
//! # Resolution is validation-based and prefix-agnostic
//!
//! A candidate token is a reference **iff it exactly equals a real yak id**
//! ([`resolve`]). Nothing keys off the configured prefix, which is what lets a
//! herd mid-migration (a mix of `yak-` and `yaksrs-` ids) keep linking
//! correctly, and why bare-hex shorthand like `0af1` is intentionally *not*
//! resolved — agents are nudged to write the full `prefix-0af1` form (skill
//! yaks-3563). The set of "real ids" is supplied by the caller as a predicate,
//! so it can widen from one herd to a friend-set (yaks-15f7) without any change
//! here.

use std::collections::HashSet;

/// A char that can appear inside a reference token: lowercase ASCII, a digit,
/// `-`, or `.` (dotted legacy ids stay a single token).
pub fn is_ref_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.'
}

/// Rewrite `[[yak-0af1]]` wiki-links to bare `yak-0af1` for uniform handling.
/// A pure text normalization: it removes the bracket pairs without touching
/// anything else, so char offsets after it map onto the displayed text.
pub fn strip_wikilinks(text: &str) -> String {
    text.replace("[[", "").replace("]]", "")
}

/// Cheap shape pre-filter: does `token` have the `<prefix>-<tail>` form of a
/// yak id? Both sides must be non-empty, the prefix `[a-z0-9]` and the tail made
/// of ref chars. This only screens *candidates* / validates a rename target; the
/// authoritative membership test is still the real id set.
pub fn has_ref_shape(token: &str) -> bool {
    match token.split_once('-') {
        Some((prefix, tail)) => {
            !prefix.is_empty()
                && !tail.is_empty()
                && prefix
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                && tail.chars().all(is_ref_char)
        }
        None => false,
    }
}

/// From `chars[i]` (which must be a ref char), scan the maximal ref-token, then
/// trim any trailing `.` off it. Real yak ids never end in a dot, so a
/// sentence-ending period (`...yak-0af1.`) is punctuation, not part of the id;
/// a legacy dotted id keeps its interior dots (`yak-a1b2.1`). Returns
/// `(token, token_end, run_end)`: `token_end` is just past the trimmed token,
/// `run_end` just past the whole run including the trailing dots, so callers can
/// re-emit those dots verbatim. The single tokenizer behind [`scan`],
/// [`rewrite`], and the TUI detail scan, so all three agree on boundaries.
pub fn token_at(chars: &[char], i: usize) -> (String, usize, usize) {
    let mut run_end = i;
    while run_end < chars.len() && is_ref_char(chars[run_end]) {
        run_end += 1;
    }
    let mut end = run_end;
    while end > i && chars[end - 1] == '.' {
        end -= 1;
    }
    let tok: String = chars[i..end].iter().collect();
    (tok, end, run_end)
}

/// Resolve a single candidate `token` to a canonical yak id. `known` reports
/// whether a given string is a real yak id. Prefix-agnostic and
/// validation-based: the token resolves iff `known(token)` holds.
pub fn resolve(token: &str, known: impl Fn(&str) -> bool) -> Option<String> {
    known(token).then(|| token.to_string())
}

/// One detected reference within a run of text: char offsets into that text
/// (not bytes), plus the resolved canonical id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefMatch {
    /// Char index where the reference token starts.
    pub start: usize,
    /// Char length of the token.
    pub len: usize,
    /// The resolved yak id.
    pub id: String,
}

/// Scan a run of text for yak references, keeping only tokens that `known`
/// confirms are real ids. Offsets are char indices into `text`; normalize wiki
/// brackets with [`strip_wikilinks`] first if you want them ignored.
pub fn scan(text: &str, known: impl Fn(&str) -> bool) -> Vec<RefMatch> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !is_ref_char(chars[i]) {
            i += 1;
            continue;
        }
        let (tok, end, run_end) = token_at(&chars, i);
        if !tok.is_empty() && known(&tok) {
            out.push(RefMatch {
                start: i,
                len: end - i,
                id: tok,
            });
        }
        i = run_end;
    }
    out
}

/// Convenience: build a predicate from a set of id string-slices.
pub fn known_from<'a>(ids: &'a HashSet<&'a str>) -> impl Fn(&str) -> bool + 'a {
    move |t: &str| ids.contains(t)
}

/// Rewrite whole reference tokens in `text`: for each maximal ref-token,
/// substitute it when `replace` returns `Some(new)`, otherwise leave it as-is.
/// Only complete tokens are considered, so a longer id that merely contains the
/// old one as a substring is untouched, and all non-token text — whitespace,
/// punctuation, and surrounding `[[ ]]` — is preserved verbatim. Returns the
/// rewritten text and whether anything changed. This is the referential-integrity
/// primitive the rename tools rewrite references with.
pub fn rewrite(text: &str, replace: impl Fn(&str) -> Option<String>) -> (String, bool) {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut changed = false;
    let mut i = 0;
    while i < chars.len() {
        if !is_ref_char(chars[i]) {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let (tok, end, run_end) = token_at(&chars, i);
        match if tok.is_empty() { None } else { replace(&tok) } {
            Some(rep) => {
                out.push_str(&rep);
                changed = true;
            }
            None => {
                for &c in &chars[i..end] {
                    out.push(c);
                }
            }
        }
        // Re-emit any trailing dots trimmed off the token as literal text.
        for &c in &chars[end..run_end] {
            out.push(c);
        }
        i = run_end;
    }
    (out, changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&'static str]) -> HashSet<&'static str> {
        list.iter().copied().collect()
    }

    #[test]
    fn resolve_is_validation_based_not_prefix_based() {
        let set = ids(&["yak-0af1", "yaksrs-15f7"]);
        let known = known_from(&set);
        // Both prefixes resolve because both ids are real — prefix-agnostic.
        assert_eq!(resolve("yak-0af1", &known), Some("yak-0af1".into()));
        assert_eq!(resolve("yaksrs-15f7", &known), Some("yaksrs-15f7".into()));
        // Bare-hex shorthand does not resolve, even though it's a real tail.
        assert_eq!(resolve("0af1", &known), None);
        // An id-shaped token that isn't a real id does not resolve.
        assert_eq!(resolve("yak-9999", &known), None);
    }

    #[test]
    fn scan_finds_only_known_ids_in_order() {
        let set = ids(&["yak-0003", "yak-0002"]);
        let got = scan("see yak-0003 and yak-0002 not yak-9999", known_from(&set));
        let seq: Vec<&str> = got.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(seq, vec!["yak-0003", "yak-0002"]);
    }

    #[test]
    fn scan_offsets_point_at_the_id_in_chars() {
        let set = ids(&["yak-0001"]);
        // Leading multi-byte char so byte and char offsets differ.
        let text = "é yak-0001 z";
        let m = &scan(text, known_from(&set))[0];
        let slice: String = text.chars().skip(m.start).take(m.len).collect();
        assert_eq!(slice, "yak-0001");
    }

    #[test]
    fn wikilinks_normalize_to_bare_refs() {
        let set = ids(&["yak-0002"]);
        let line = strip_wikilinks("a [[yak-0002]] b");
        assert_eq!(line, "a yak-0002 b");
        assert_eq!(scan(&line, known_from(&set)).len(), 1);
    }

    #[test]
    fn has_ref_shape_screens_candidates() {
        assert!(has_ref_shape("yak-0af1"));
        assert!(has_ref_shape("yaksrs-15f7"));
        assert!(!has_ref_shape("0af1")); // no prefix
        assert!(!has_ref_shape("yak-")); // empty tail
        assert!(!has_ref_shape("plainword"));
    }

    #[test]
    fn rewrite_touches_only_whole_tokens_and_keeps_brackets() {
        let repl = |t: &str| (t == "yaksrs-0001").then(|| "yak-0001".to_string());
        let (out, changed) = rewrite("see yaksrs-0001 and [[yaksrs-0001]] not yaksrs-00019", repl);
        assert!(changed);
        assert_eq!(out, "see yak-0001 and [[yak-0001]] not yaksrs-00019");
    }

    #[test]
    fn rewrite_reports_no_change_when_absent() {
        let (out, changed) = rewrite("nothing here", |t: &str| (t == "x").then(|| "y".into()));
        assert!(!changed);
        assert_eq!(out, "nothing here");
    }

    #[test]
    fn trailing_period_is_not_part_of_the_id() {
        let set = ids(&["yaksrs-688d"]);
        let got = scan("described in yaksrs-688d.", known_from(&set));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "yaksrs-688d");
        assert_eq!(got[0].len, "yaksrs-688d".chars().count()); // span excludes the '.'
    }

    #[test]
    fn rewrite_preserves_a_trailing_period() {
        let (out, changed) = rewrite("Split out of yaksrs-78dc.", |t: &str| {
            (t == "yaksrs-78dc").then(|| "yaks-78dc".to_string())
        });
        assert!(changed);
        assert_eq!(out, "Split out of yaks-78dc.");
    }

    #[test]
    fn interior_dot_in_a_legacy_id_is_kept() {
        let set = ids(&["yak-a1b2.1"]);
        let got = scan("see yak-a1b2.1.", known_from(&set));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "yak-a1b2.1");
    }

    #[test]
    fn reserved_qualifier_colon_scans_local_id_for_now() {
        // Until yaks-15f7 lands, the herd qualifier is not parsed; the local id
        // still resolves and the `other:` part is simply not a ref char run.
        let set = ids(&["yak-0001"]);
        let got = scan("other:yak-0001", known_from(&set));
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "yak-0001");
    }
}
