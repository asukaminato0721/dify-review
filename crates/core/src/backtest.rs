//! Calibration: had a rule been running, where would it have commented,
//! compared with where the reviewer did?
//!
//! Positives are chunks (a file's hunks, judged together) the reviewer
//! commented on, labelled with the rule
//! (and, for a choice rule, the fix) the comment is an instance of.
//! Negatives are chunks the reviewer let through: the final diff of the
//! pull requests they reviewed. A reviewer does not comment on every
//! instance, so a negative that does contain the rule's hint pattern is
//! counted as *tolerated*, not noise; noise is a comment on a negative the
//! hint does not match (or any negative, for a rule without a hint).
//!
//! Feedback on live comments adds both kinds: a comment that turned out
//! right is a positive; one that turned out wrong (👎, a reply rejecting
//! it) is an explicit negative for its rule, counted as noise whether or
//! not the hint matches.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diff::Chunk;
use crate::review::{Answer, decide_at, passes_gate};
use crate::rules::{OptionName, Rule, RuleId};

/// Thresholds tried for every rule.
/// Low values matter for choice rules: confidence is spread over options.
pub const GRID: [f64; 14] = [
    0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.6, 0.65, 0.7, 0.75, 0.8, 0.85, 0.9, 0.95,
];
/// Precision is weighted over recall: a wrong comment costs more trust
/// than a missed one.
const BETA: f64 = 0.5;
/// A threshold is only suggested if at least this share of the comments it
/// produces are hits.
pub const MIN_PRECISION: f64 = 0.5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    pub rule: RuleId,
    #[serde(default)]
    pub option: Option<OptionName>,
}

#[derive(Debug, Clone)]
pub struct Sample {
    /// Where it came from: the review comment URL (positive) or the pull
    /// request, file and first hunk header (negative).
    pub origin: String,
    pub path: String,
    pub chunk: Chunk,
    /// Empty for a negative.
    pub labels: Vec<Label>,
    /// Rules that commented here and were told they were wrong.
    pub rejected: Vec<RuleId>,
    /// Jev's answers keyed by rule id; a rule not asked is absent.
    pub answers: BTreeMap<String, Answer>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub threshold: f64,
    /// Positives the rule would have commented on.
    pub hits: usize,
    /// Of those, how many with the labelled fix (choice rules).
    pub right_fix: usize,
    /// Negatives without the hint pattern the rule would have commented on.
    pub noise: usize,
    /// Negatives with the hint pattern it would have commented on: what the
    /// reviewer let through at the time.
    pub tolerated: usize,
}

impl Row {
    pub fn precision(&self) -> Option<f64> {
        let fired = self.hits + self.noise;
        (fired > 0).then(|| self.hits as f64 / fired as f64)
    }

    fn f_beta(&self, positives: usize) -> f64 {
        let p = self.precision().unwrap_or(0.0);
        let r = match positives {
            0 => 0.0,
            n => self.hits as f64 / n as f64,
        };
        let b2 = BETA * BETA;
        match p + r {
            0.0 => 0.0,
            _ => (1.0 + b2) * p * r / (b2 * p + r),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RuleReport {
    pub rule: RuleId,
    pub current: f64,
    pub positives: usize,
    /// Positives whose path the rule's `paths` do not cover: never asked.
    pub outside_paths: usize,
    pub negatives: usize,
    /// Explicit negatives: live comments of this rule judged wrong.
    pub rejected: usize,
    pub rows: Vec<Row>,
    /// The threshold with the best F0.5 among those with precision of at
    /// least MIN_PRECISION, higher on ties; None when there is none: the
    /// rule's question needs rewriting.
    pub suggested: Option<f64>,
    /// At the suggested threshold (else the current one), by origin.
    pub missed: Vec<String>,
    pub noise: Vec<String>,
    pub tolerated: Vec<String>,
}

pub fn evaluate(rule: &Rule, samples: &[Sample]) -> RuleReport {
    let label_of = |s: &Sample| s.labels.iter().find(|l| l.rule == rule.id).cloned();
    let positives: Vec<(&Sample, Label)> = samples
        .iter()
        .filter_map(|s| label_of(s).map(|l| (s, l)))
        .collect();
    let negatives: Vec<&Sample> = samples
        .iter()
        .filter(|s| s.labels.is_empty() && s.rejected.is_empty() && rule.applies_to(&s.path))
        .collect();
    let rejected: Vec<&Sample> = samples
        .iter()
        .filter(|s| s.rejected.contains(&rule.id))
        .collect();
    let hinted = |s: &Sample| {
        rule.hint
            .as_ref()
            .is_some_and(|h| s.chunk.added().any(|l| h.is_match(&l.text)))
    };
    let verdict = |s: &Sample, t: f64| {
        s.answers
            .get(&rule.id.0)
            .and_then(|a| decide_at(rule, &s.chunk, a, t))
    };
    let rows: Vec<Row> = GRID
        .iter()
        .map(|&t| {
            let fired: Vec<_> = positives
                .iter()
                .filter_map(|(s, l)| verdict(s, t).map(|v| (v, l)))
                .collect();
            Row {
                threshold: t,
                hits: fired.len(),
                right_fix: fired.iter().filter(|(v, l)| v.option == l.option).count(),
                noise: negatives
                    .iter()
                    .filter(|s| !hinted(s) && verdict(s, t).is_some())
                    .count()
                    + rejected.iter().filter(|s| verdict(s, t).is_some()).count(),
                tolerated: negatives
                    .iter()
                    .filter(|s| hinted(s) && verdict(s, t).is_some())
                    .count(),
            }
        })
        .collect();
    let suggested = rows
        .iter()
        .filter(|r| r.hits > 0 && r.precision().is_some_and(|p| p >= MIN_PRECISION))
        .max_by(|a, b| {
            a.f_beta(positives.len())
                .total_cmp(&b.f_beta(positives.len()))
                .then(a.threshold.total_cmp(&b.threshold))
        })
        .map(|r| r.threshold);
    let at = suggested.unwrap_or(rule.threshold);
    let missed = positives
        .iter()
        .filter(|(s, _)| verdict(s, at).is_none())
        .map(|(s, l)| {
            let why = match s.answers.get(&rule.id.0) {
                None => "not asked".to_owned(),
                Some(_) if !passes_gate(rule, &s.chunk) => "hint not matched in scope".to_owned(),
                Some(Answer::Noul(p)) => format!("jev {p:.2}"),
                Some(Answer::Choice { choice, confidence }) => {
                    format!("jev {choice} {confidence:.2}")
                }
            };
            let want = l
                .option
                .as_ref()
                .map(|o| format!(", labelled {o}"))
                .unwrap_or_default();
            format!("{} ({why}{want})", s.origin)
        })
        .collect();
    let fired_at = |want_hint: bool| {
        negatives
            .iter()
            .filter(|s| hinted(s) == want_hint && verdict(s, at).is_some())
            .map(|s| s.origin.clone())
            .collect::<Vec<_>>()
    };
    let noise = fired_at(false)
        .into_iter()
        .chain(
            rejected
                .iter()
                .filter(|s| verdict(s, at).is_some())
                .map(|s| format!("{} (judged wrong live)", s.origin)),
        )
        .collect();
    RuleReport {
        missed,
        noise,
        tolerated: fired_at(true),
        rule: rule.id.clone(),
        current: rule.threshold,
        positives: positives.len(),
        outside_paths: positives
            .iter()
            .filter(|(s, _)| !rule.applies_to(&s.path))
            .count(),
        negatives: negatives.len(),
        rejected: rejected.len(),
        rows,
        suggested,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::parse_rule;
    use crate::rules::tests::NOUL;

    fn sample(path: &str, label: bool, p: f64) -> Sample {
        sample_with(path, label, p, "+x = 1")
    }

    fn sample_with(path: &str, label: bool, p: f64, line: &str) -> Sample {
        Sample {
            origin: path.into(),
            path: path.into(),
            chunk: crate::review::tests::chunk(&format!("@@ -1 +1 @@\n{line}")),
            rejected: Vec::new(),
            labels: label
                .then(|| Label {
                    rule: RuleId("r".into()),
                    option: None,
                })
                .into_iter()
                .collect(),
            answers: [("r".to_owned(), Answer::Noul(p))].into(),
        }
    }

    #[test]
    fn sweeps_and_prefers_precision() {
        let rule = parse_rule("r", NOUL).unwrap();
        let samples = vec![
            sample("api/a.py", true, 0.95),
            sample("api/b.py", true, 0.75),
            sample("api/c.py", false, 0.72),
            sample("api/d.py", false, 0.3),
            // Outside the rule's paths: not a negative for it.
            sample("web/e.ts", false, 0.99),
        ];
        let r = evaluate(&rule, &samples);
        assert_eq!((r.positives, r.negatives, r.outside_paths), (2, 2, 0));
        let at = |t: f64| r.rows.iter().find(|row| row.threshold == t).unwrap();
        assert_eq!((at(0.7).hits, at(0.7).noise), (2, 1));
        assert_eq!((at(0.75).hits, at(0.75).noise), (2, 0));
        assert_eq!(r.suggested, Some(0.75));
        assert!(r.missed.is_empty() && r.noise.is_empty());
    }

    #[test]
    fn hinted_negatives_are_tolerated_not_noise() {
        let rule = parse_rule("r", NOUL).unwrap();
        let samples = vec![
            sample("api/a.py", true, 0.9),
            sample_with("api/b.py", false, 0.9, "+x = f()  # type: ignore"),
            sample("api/c.py", false, 0.9),
        ];
        let r = evaluate(&rule, &samples);
        let row = r.rows.iter().find(|row| row.threshold == 0.7).unwrap();
        assert_eq!((row.hits, row.noise, row.tolerated), (1, 1, 1));
        assert_eq!(r.tolerated, vec!["api/b.py".to_owned()]);
    }

    #[test]
    fn rejected_counts_as_noise_even_with_the_pattern() {
        let rule = parse_rule("r", NOUL).unwrap();
        let mut wrong = sample_with("api/b.py", false, 0.9, "+x = f()  # type: ignore");
        wrong.rejected = vec![RuleId("r".into())];
        let r = evaluate(&rule, &[sample("api/a.py", true, 0.9), wrong]);
        let row = r.rows.iter().find(|row| row.threshold == 0.7).unwrap();
        assert_eq!(
            (row.hits, row.noise, row.tolerated, r.rejected),
            (1, 1, 0, 1)
        );
    }

    #[test]
    fn never_hitting_suggests_nothing() {
        let rule = parse_rule("r", NOUL).unwrap();
        let r = evaluate(&rule, &[sample("api/a.py", true, 0.1)]);
        assert_eq!(r.suggested, None);
    }
}
