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
    fn rejects_garbage() {
        assert!(parse_patch("+orphan").is_err());
        assert!(parse_patch("@@ nonsense @@").is_err());
        assert!(parse_patch("").unwrap().is_empty());
    }
}
