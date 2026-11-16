//! From a rule and a hunk to an inline comment: decide from Jev's answer
//! (or the hint alone), anchor on a line, render with a dedupe marker.

use serde::{Deserialize, Serialize};

use crate::MARKER;
use crate::diff::{Hunk, LineKind, NewLine};
use crate::rules::{HintScope, Judge, OptionName, Rule, RuleId, Severity};

/// Jev's answer to one rule's question.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Answer {
    Noul(f64),
    Choice { choice: OptionName, confidence: f64 },
}

/// What a rule says about a hunk, before it is placed on a line.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Verdict {
    pub option: Option<OptionName>,
    /// Probability (noul) or confidence (choice); 1.0 for a hint match.
    pub score: f64,
    pub comment: String,
}

/// Decide about a hunk from Jev's answer at the rule's own threshold.
pub fn decide(rule: &Rule, hunk: &Hunk, answer: &Answer) -> Option<Verdict> {
    decide_at(rule, hunk, answer, rule.threshold)
}

/// A gated rule only speaks where its hint matches an added line.
pub fn passes_gate(rule: &Rule, hunk: &Hunk) -> bool {
    !rule.gate || hint_match(rule, hunk).is_some()
}

/// Decide at a given threshold: the probability of yes (noul) or the
/// confidence in a commenting option (choice). An answer of the wrong shape
/// for the rule is ignored (no comment) rather than guessed at.
pub fn decide_at(rule: &Rule, hunk: &Hunk, answer: &Answer, threshold: f64) -> Option<Verdict> {
    match (&rule.judge, answer) {
        _ if !passes_gate(rule, hunk) => None,
        (Judge::Noul { comment, .. }, Answer::Noul(p)) => (*p >= threshold).then(|| Verdict {
            option: None,
            score: *p,
            comment: comment.clone(),
        }),
        (Judge::Choice { options, .. }, Answer::Choice { choice, confidence }) => options
            .iter()
            .find(|o| &o.name == choice)
            .and_then(|o| o.comment.clone())
            .filter(|_| *confidence >= threshold)
            .map(|comment| Verdict {
                option: Some(choice.clone()),
                score: *confidence,
                comment,
            }),
        _ => None,
    }
}

/// Without Jev: a noul rule comments when its hint matches an added line.
/// Choice rules need Jev to pick the fix, so they stay silent.
pub fn decide_by_hint(rule: &Rule, hunk: &Hunk) -> Option<Verdict> {
    match &rule.judge {
        Judge::Noul { comment, .. } => hint_match(rule, hunk).map(|_| Verdict {
            option: None,
            score: 1.0,
            comment: comment.clone(),
        }),
        Judge::Choice { .. } => None,
    }
}

/// The first line in the rule's hint scope the hint matches. Removed lines
/// never count: a comment can only sit on the new side.
fn hint_match<'h>(rule: &Rule, hunk: &'h Hunk) -> Option<(NewLine, &'h str)> {
    let hint = rule.hint.as_ref()?;
    let in_scope = |kind: LineKind| match rule.hint_scope {
        HintScope::Added => kind == LineKind::Added,
        HintScope::Hunk => kind != LineKind::Removed,
    };
    hunk.lines
        .iter()
        .filter(|l| in_scope(l.kind))
        .find(|l| hint.is_match(&l.text))
        .and_then(|l| l.new_line.map(|n| (n, l.text.as_str())))
}

/// Where the comment goes: the first added line the hint matches, else the
/// first added line of the hunk.
pub fn anchor<'h>(rule: &Rule, hunk: &'h Hunk) -> Option<(NewLine, &'h str)> {
    hint_match(rule, hunk).or_else(|| {
        hunk.added()
            .next()
            .and_then(|l| l.new_line.map(|n| (n, l.text.as_str())))
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule: RuleId,
    pub severity: Severity,
    pub path: String,
    pub line: NewLine,
    pub line_text: String,
    pub verdict: Verdict,
    pub example: Option<String>,
}

impl Finding {
    pub fn new(rule: &Rule, path: &str, hunk: &Hunk, verdict: Verdict) -> Option<Self> {
        let (line, text) = anchor(rule, hunk)?;
        Some(Self {
            rule: rule.id.clone(),
            severity: rule.severity,
            path: path.to_owned(),
            line,
            line_text: text.to_owned(),
            verdict,
            example: rule.examples.first().cloned(),
        })
    }

    /// Stable across pushes that only move the line: rule, path and the
    /// line's trimmed text.
    pub fn key(&self) -> String {
        let input = format!("{}\0{}\0{}", self.rule, self.path, self.line_text.trim());
        format!("{:016x}", fnv1a(input.as_bytes()))
    }

    pub fn body(&self) -> String {
        let source = self
            .example
            .as_deref()
            .map(|u| format!(" Learned from [an earlier review]({u})."))
            .unwrap_or_default();
        format!(
            "{}\n\n<sub>dify-codereview · `{}`{}{} Reply here if this is wrong.</sub>\n{}:rule={}{};key={} -->",
            self.verdict.comment,
            self.rule,
            self.verdict
                .option
                .as_ref()
                .map(|o| format!(" ({o})"))
                .unwrap_or_default(),
            source,
            MARKER,
            self.rule,
            self.verdict
                .option
                .as_ref()
                .map(|o| format!(";option={o}"))
                .unwrap_or_default(),
            self.key(),
        )
    }
}

/// The dedupe keys in a comment body dify-review posted earlier.
pub fn marker_keys(body: &str) -> Vec<String> {
    marker_values(body, "key")
}

/// Values of `field` in every `<!-- dify-review:a=1;b=2 -->` marker of a body.
pub fn marker_values(body: &str, field: &str) -> Vec<String> {
    let prefix = format!("{field}=");
    body.match_indices(MARKER)
        .filter_map(|(i, _)| {
            let rest = &body[i + MARKER.len()..];
            let end = rest.find("-->")?;
            rest[..end]
                .trim_start_matches(':')
                .split(';')
                .find_map(|kv| kv.trim().strip_prefix(prefix.as_str()))
                .map(|v| v.trim().to_owned())
        })
        .collect()
}

/// A new comment needs a score this far above the rule's threshold. Jev's
/// probabilities move a little between identical calls; without a margin a
/// borderline hunk comments on one run and not the next.
pub const NEW_COMMENT_MARGIN: f64 = 0.05;

/// The bar a new comment must reach under `rule`.
pub fn posting_threshold(rule: &Rule) -> f64 {
    (rule.threshold + NEW_COMMENT_MARGIN).min(1.0)
}

/// Marker for a review summary: the head it judged and, when someone asked
/// for it, the comment that asked.
pub fn summary_marker(head: &str, trigger: Option<u64>) -> String {
    let trigger = trigger.map(|t| format!(";trigger={t}")).unwrap_or_default();
    format!("{MARKER}:head={head}{trigger} -->")
}

/// Whether `body` mentions `handle` (e.g. `@dify-review`) as a whole word:
/// not inside an e-mail address and not as the prefix of a longer handle.
pub fn mentions(body: &str, handle: &str) -> bool {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    let lower = body.to_ascii_lowercase();
    let handle = handle.to_ascii_lowercase();
    lower.match_indices(&handle).any(|(i, _)| {
        let before = lower[..i].chars().next_back();
        let after = lower[i + handle.len()..].chars().next();
        !before.is_some_and(word) && !after.is_some_and(word)
    })
}

/// FNV-1a: a small hash that is stable across Rust versions and platforms.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::parse_patch;
    use crate::rules::parse_rule;
    use crate::rules::tests::{CHOICE, NOUL};

    fn setup() -> (Vec<Rule>, Hunk) {
        let rules = vec![
            parse_rule("no-type-ignore", NOUL).unwrap(),
            parse_rule("doubles", CHOICE).unwrap(),
        ];
        let hunk = parse_patch("@@ -1,2 +1,3 @@\n a = 1\n+b = 2\n+c = f()  # type: ignore\n d")
            .unwrap()
            .remove(0);
        (rules, hunk)
    }

    #[test]
    fn noul_threshold_and_shape() {
        let (rules, hunk) = setup();
        assert!(decide(&rules[0], &hunk, &Answer::Noul(0.92)).is_some());
        assert!(decide(&rules[0], &hunk, &Answer::Noul(0.5)).is_none());
        let wrong = Answer::Choice {
            choice: OptionName("real".into()),
            confidence: 0.99,
        };
        assert!(decide(&rules[0], &hunk, &wrong).is_none());
    }

    #[test]
    fn choice_comments_only_for_commenting_options() {
        let (rules, hunk) = setup();
        let pick = |name: &str, c| Answer::Choice {
            choice: OptionName(name.into()),
            confidence: c,
        };
        let v = decide(&rules[1], &hunk, &pick("real", 0.9)).unwrap();
        assert_eq!(v.option, Some(OptionName("real".into())));
        assert_eq!(v.comment, "Use the real class.");
        assert!(decide(&rules[1], &hunk, &pick("none", 0.99)).is_none());
        assert!(decide(&rules[1], &hunk, &pick("real", 0.3)).is_none());
        assert!(decide(&rules[1], &hunk, &pick("unknown", 0.99)).is_none());
    }

    #[test]
    fn anchors_on_hint_else_first_added_line() {
        let (rules, hunk) = setup();
        assert_eq!(anchor(&rules[0], &hunk).unwrap().0, NewLine(3));
        assert_eq!(anchor(&rules[1], &hunk).unwrap().0, NewLine(2));
        assert!(decide_by_hint(&rules[0], &hunk).is_some());
        assert!(decide_by_hint(&rules[1], &hunk).is_none());
    }

    #[test]
    fn hint_scope_hunk_also_matches_context_lines() {
        let gated = |scope: &str| {
            let text = format!(
                "severity = 'nit'\npaths = ['**']\ngate = true\nhint_scope = '{scope}'\nhint = 'MagicMock'\n[noul]\nquestion = 'q'\ncomment = 'c'\n"
            );
            crate::rules::parse_rule("r", &text).unwrap()
        };
        let hunk = parse_patch("@@ -1,2 +1,3 @@\n m = MagicMock()\n+x: int = 1\n y")
            .unwrap()
            .remove(0);
        assert!(!passes_gate(&gated("added"), &hunk));
        assert!(passes_gate(&gated("hunk"), &hunk));
        assert_eq!(anchor(&gated("hunk"), &hunk).unwrap().0, NewLine(1));
    }

    #[test]
    fn mention_is_a_whole_handle() {
        assert!(mentions("@dify-review please", "@dify-review"));
        assert!(mentions("cc @Dify-Review.", "@dify-review"));
        assert!(!mentions("@dify-review-bot", "@dify-review"));
        assert!(!mentions("ops@dify-review", "@dify-review"));
        assert!(!mentions("dify-review", "@dify-review"));
    }

    #[test]
    fn summary_marker_round_trips() {
        let body = format!("text\n{}", summary_marker("abc123", Some(42)));
        assert_eq!(marker_values(&body, "head"), vec!["abc123".to_owned()]);
        assert_eq!(marker_values(&body, "trigger"), vec!["42".to_owned()]);
        assert!(marker_values(&summary_marker("abc", None), "trigger").is_empty());
    }

    #[test]
    fn body_round_trips_its_key() {
        let (rules, hunk) = setup();
        let v = decide(&rules[0], &hunk, &Answer::Noul(0.9)).unwrap();
        let f = Finding::new(&rules[0], "api/x.py", &hunk, v).unwrap();
        let body = f.body();
        assert!(body.starts_with("Fix the type."));
        assert!(body.contains("https://example.test/r1"));
        assert_eq!(marker_keys(&body), vec![f.key()]);
        assert!(marker_keys("no marker here").is_empty());
    }
}
