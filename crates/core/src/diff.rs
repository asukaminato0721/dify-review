//! Unified diff hunks as GitHub returns them in a file's `patch`.

use std::fmt;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// A line number in the new version of a file: what GitHub review comments
/// address with `side: RIGHT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NewLine(pub u32);

impl fmt::Display for NewLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Added,
    Context,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: LineKind,
    /// None for removed lines, which have no place in the new file.
    pub new_line: Option<NewLine>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

impl Hunk {
    pub fn added(&self) -> impl Iterator<Item = &DiffLine> {
        self.lines.iter().filter(|l| l.kind == LineKind::Added)
    }

    pub fn has_additions(&self) -> bool {
        self.added().next().is_some()
    }

    /// The hunk as diff text, the way a reviewer reads it.
    pub fn render(&self) -> String {
        let body = self.lines.iter().map(|l| {
            let prefix = match l.kind {
                LineKind::Added => '+',
                LineKind::Context => ' ',
                LineKind::Removed => '-',
            };
            format!("{prefix}{}", l.text)
        });
        std::iter::once(self.header.clone())
            .chain(body)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Hunks of one file judged together, in one Jev request: the rules'
/// questions, most of a request, are then sent once per file rather than
/// once per hunk. Built only by `Chunk::pack`: every hunk has an added line
/// (somewhere to comment), and the chunk fits the size it was packed for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    hunks: Vec<Hunk>,
    /// Length of `render()`.
    chars: usize,
}

/// A file's hunks packed into chunks; a hunk over the limit on its own is
/// set apart, to be skipped rather than truncated into a wrong answer.
#[derive(Debug, Default)]
pub struct Packed {
    pub chunks: Vec<Chunk>,
    pub oversized: Vec<Hunk>,
}

impl Chunk {
    /// Consecutive hunks with additions, as few chunks as fit `max_chars`.
    pub fn pack(hunks: impl IntoIterator<Item = Hunk>, max_chars: usize) -> Packed {
        hunks
            .into_iter()
            .filter(Hunk::has_additions)
            .fold(Packed::default(), |mut p, h| {
                let size = h.render().len();
                match (size > max_chars, p.chunks.last_mut()) {
                    (true, _) => p.oversized.push(h),
                    // +1: the newline joining it to the chunk.
                    (false, Some(c)) if c.chars + 1 + size <= max_chars => {
                        c.chars += 1 + size;
                        c.hunks.push(h);
                    }
                    (false, _) => p.chunks.push(Chunk {
                        hunks: vec![h],
                        chars: size,
                    }),
                }
                p
            })
    }

    pub fn hunks(&self) -> &[Hunk] {
        &self.hunks
    }

    /// Never empty: `pack` starts a chunk with a hunk.
    pub fn first(&self) -> &Hunk {
        &self.hunks[0]
    }

    pub fn lines(&self) -> impl Iterator<Item = &DiffLine> {
        self.hunks.iter().flat_map(|h| h.lines.iter())
    }

    pub fn added(&self) -> impl Iterator<Item = &DiffLine> {
        self.hunks.iter().flat_map(Hunk::added)
    }

    /// The hunks as diff text, one after another.
    pub fn render(&self) -> String {
        self.hunks
            .iter()
            .map(Hunk::render)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Parse a file patch into hunks, numbering every added and context line by
/// its position in the new file.
pub fn parse_patch(patch: &str) -> Result<Vec<Hunk>> {
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut next = 0u32;
    patch
        .lines()
        .enumerate()
        .try_for_each(|(i, raw)| -> Result<()> {
            let n = i + 1;
            match raw.chars().next() {
                Some('@') if raw.starts_with("@@") => {
                    next = new_start(raw)
                        .with_context(|| format!("patch line {n}: bad hunk header {raw:?}"))?;
                    hunks.push(Hunk {
                        header: raw.to_owned(),
                        lines: Vec::new(),
                    });
                    Ok(())
                }
                // "\ No newline at end of file"
                Some('\\') => Ok(()),
                first => {
                    let kind = match first {
                        Some('+') => LineKind::Added,
                        Some('-') => LineKind::Removed,
                        Some(' ') | None => LineKind::Context,
                        Some(_) => bail!("patch line {n}: unexpected {raw:?}"),
                    };
                    let hunk = hunks
                        .last_mut()
                        .with_context(|| format!("patch line {n}: content before any @@ header"))?;
                    let in_new_file = kind != LineKind::Removed;
                    hunk.lines.push(DiffLine {
                        kind,
                        new_line: in_new_file.then_some(NewLine(next)),
                        text: raw.get(1..).unwrap_or_default().to_owned(),
                    });
                    next += u32::from(in_new_file);
                    Ok(())
                }
            }
        })?;
    Ok(hunks)
}

/// `@@ -a,b +c,d @@ ...` -> c
fn new_start(header: &str) -> Option<u32> {
    let plus = header.split_whitespace().nth(2)?.strip_prefix('+')?;
    plus.split(',').next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATCH: &str = "@@ -10,4 +10,5 @@ def f():\n     a = 1\n-    b = 2\n+    b = 3\n+    c = 4\n     d = 5\n\\ No newline at end of file\n@@ -40 +41,2 @@\n+x\n y";

    #[test]
    fn numbers_new_lines() {
        let hunks = parse_patch(PATCH).unwrap();
        assert_eq!(hunks.len(), 2);
        let lines: Vec<_> = hunks[0]
            .lines
            .iter()
            .map(|l| (l.kind, l.new_line.map(|n| n.0), l.text.as_str()))
            .collect();
        assert_eq!(
            lines,
            [
                (LineKind::Context, Some(10), "    a = 1"),
                (LineKind::Removed, None, "    b = 2"),
                (LineKind::Added, Some(11), "    b = 3"),
                (LineKind::Added, Some(12), "    c = 4"),
                (LineKind::Context, Some(13), "    d = 5"),
            ]
        );
        assert_eq!(
            hunks[1]
                .added()
                .map(|l| l.new_line.unwrap().0)
                .collect::<Vec<_>>(),
            [41]
        );
        assert!(hunks[1].render().starts_with("@@ -40 +41,2 @@\n+x\n y"));
    }

    #[test]
    fn packs_hunks_with_additions_up_to_the_limit() {
        let hunk = |n: u32, body: &str| {
            parse_patch(&format!("@@ -{n} +{n} @@\n{body}"))
                .unwrap()
                .remove(0)
        };
        let (a, b, gone, big) = (
            hunk(1, "+a"),
            hunk(5, "+b"),
            hunk(9, "-gone"),
            hunk(20, &format!("+{}", "x".repeat(40))),
        );
        let p = Chunk::pack([a.clone(), gone, b.clone(), big.clone()], 30);
        assert_eq!(p.oversized, vec![big]);
        assert_eq!(p.chunks.len(), 1);
        assert_eq!(p.chunks[0].hunks(), [a.clone(), b.clone()]);
        assert_eq!(
            p.chunks[0].render(),
            format!("{}\n{}", a.render(), b.render())
        );
        assert_eq!(p.chunks[0].render().len(), p.chunks[0].chars);
        // Too small for both: one chunk each.
        assert_eq!(Chunk::pack([a, b], 15).chunks.len(), 2);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_patch("+orphan").is_err());
        assert!(parse_patch("@@ nonsense @@").is_err());
        assert!(parse_patch("").unwrap().is_empty());
    }
}
