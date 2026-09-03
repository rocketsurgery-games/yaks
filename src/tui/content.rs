//! Split a task `body` into its description + comment blocks and stitch them
//! back together. Round-trip-safe against the on-disk shape that
//! [`crate::store::append_note`] writes:
//!
//! ```text
//! <description>
//!
//! ---
//! ▸ <timestamp>
//! <comment text>
//! ```
//!
//! A comment boundary is *only* a line that is exactly `---` immediately
//! followed by a `▸ <timestamp>` line, so a bare `---` horizontal rule inside
//! prose stays part of the description. `parse` then `assemble` reproduces an
//! `append_note`-built body byte-for-byte; `assemble` drops empty comments so
//! saving an emptied comment deletes it.

/// Marker glyph that opens a comment's timestamp line (`▸`).
const MARK: char = '\u{25b8}';

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
    Description,
    Comment {
        timestamp: String,
        /// The `[actor]` that wrote the comment, if any (see `store::note_head`).
        actor: Option<String>,
    },
}

/// One content block: its kind plus the body text (no separator/marker lines).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub kind: BlockKind,
    pub text: String,
}

/// True if line `j` opens a comment: `---` followed by a `▸ …` line.
fn is_marker(lines: &[&str], j: usize) -> bool {
    lines.get(j).is_some_and(|l| l.trim_end() == "---")
        && lines
            .get(j + 1)
            .is_some_and(|l| l.trim_start().starts_with(MARK))
}

/// Parse a body into `[Description, Comment*]`. Always returns a `Description`
/// block first (its text possibly empty), then one `Comment` per marker.
pub fn parse(body: &str) -> Vec<Block> {
    let lines: Vec<&str> = body.split('\n').collect();
    let mut blocks = Vec::new();

    // Description: everything before the first marker.
    let mut j = 0;
    while j < lines.len() && !is_marker(&lines, j) {
        j += 1;
    }
    blocks.push(Block {
        kind: BlockKind::Description,
        text: lines[..j].join("\n").trim_end().to_string(),
    });

    // Comments: each spans its `▸` line's content until the next marker.
    while j < lines.len() {
        let head = lines[j + 1].trim_start().trim_start_matches(MARK).trim();
        let (timestamp, actor) = crate::store::split_note_head(head);
        let mut k = j + 2;
        while k < lines.len() && !is_marker(&lines, k) {
            k += 1;
        }
        blocks.push(Block {
            kind: BlockKind::Comment { timestamp, actor },
            text: lines[j + 2..k].join("\n").trim_end().to_string(),
        });
        j = k;
    }
    blocks
}

/// Reassemble blocks into a body, matching `append_note`'s shape. Comments with
/// empty (whitespace-only) text are dropped.
pub fn assemble(blocks: &[Block]) -> String {
    let mut out = String::new();
    let mut wrote = false;
    for b in blocks {
        let text = b.text.trim_end();
        match &b.kind {
            BlockKind::Description => {
                out.push_str(text);
                wrote = !text.is_empty();
            }
            BlockKind::Comment { timestamp, actor } => {
                if text.is_empty() {
                    continue; // emptied comment → deleted
                }
                if wrote {
                    out.push_str("\n\n");
                }
                out.push_str("---\n");
                out.push(MARK);
                out.push(' ');
                out.push_str(&crate::store::note_head(timestamp, actor.as_deref()));
                out.push('\n');
                out.push_str(text);
                wrote = true;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::append_note;

    fn desc(text: &str) -> Block {
        Block {
            kind: BlockKind::Description,
            text: text.into(),
        }
    }
    fn comment(ts: &str, text: &str) -> Block {
        Block {
            kind: BlockKind::Comment {
                timestamp: ts.into(),
                actor: None,
            },
            text: text.into(),
        }
    }

    #[test]
    fn empty_body_is_one_empty_description() {
        assert_eq!(parse(""), vec![desc("")]);
    }

    #[test]
    fn plain_description_no_comments() {
        assert_eq!(
            parse("Just a description.\n\nMore."),
            vec![desc("Just a description.\n\nMore.")]
        );
    }

    #[test]
    fn bare_rule_stays_in_description() {
        // A `---` not followed by a `▸` line is a horizontal rule, not a comment.
        let body = "intro\n\n---\n\nafter";
        assert_eq!(parse(body), vec![desc(body)]);
    }

    #[test]
    fn splits_description_and_comments() {
        let body = append_note(
            &append_note(
                "The description.",
                "2026-01-01T00:00:00Z",
                None,
                "first note",
            ),
            "2026-01-02T00:00:00Z",
            None,
            "second note",
        );
        assert_eq!(
            parse(&body),
            vec![
                desc("The description."),
                comment("2026-01-01T00:00:00Z", "first note"),
                comment("2026-01-02T00:00:00Z", "second note"),
            ]
        );
    }

    #[test]
    fn parses_and_round_trips_the_actor_stamp() {
        // A body written with attribution (store::append_note) must parse the
        // actor as a first-class field, not glue it into the timestamp.
        let body = crate::store::append_note("Desc.", "2026-01-01T00:00:00Z", Some("opus"), "hi");
        let blocks = parse(&body);
        assert_eq!(
            blocks[1].kind,
            BlockKind::Comment {
                timestamp: "2026-01-01T00:00:00Z".into(),
                actor: Some("opus".into()),
            }
        );
        // ...and assemble puts it back exactly (byte-identical body).
        assert_eq!(assemble(&blocks), body);
    }

    #[test]
    fn comment_only_body_has_empty_description() {
        let body = append_note("", "2026-01-01T00:00:00Z", None, "hi");
        assert_eq!(
            parse(&body),
            vec![desc(""), comment("2026-01-01T00:00:00Z", "hi")]
        );
    }

    #[test]
    fn round_trips_append_note_bodies() {
        for body in [
            append_note("", "2026-01-01T00:00:00Z", None, "solo"),
            append_note("Desc.", "2026-01-01T00:00:00Z", None, "one"),
            append_note(
                &append_note("Multi\nline desc.", "2026-01-01T00:00:00Z", None, "a\nb"),
                "2026-01-02T00:00:00Z",
                None,
                "c",
            ),
        ] {
            assert_eq!(assemble(&parse(&body)), body, "round trip for:\n{body}");
        }
    }

    #[test]
    fn assemble_drops_emptied_comment() {
        let blocks = vec![
            desc("keep"),
            comment("2026-01-01T00:00:00Z", "   "), // emptied → dropped
            comment("2026-01-02T00:00:00Z", "stays"),
        ];
        assert_eq!(
            assemble(&blocks),
            "keep\n\n---\n\u{25b8} 2026-01-02T00:00:00Z\nstays"
        );
    }
}
