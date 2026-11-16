//! Rules loaded from `rules/<id>.toml` and validated once: a [`Rule`] always
//! has a compiled path matcher, a compiled hint (if any) and exactly one
//! judge. The file name is the rule id, so ids are unique by construction.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuleId(pub String);

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&self.0)
    }
}

/// Name of one option of a choice rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OptionName(pub String);

impl fmt::Display for OptionName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Nit,
    Should,
}

#[derive(Debug, Clone)]
pub struct ChoiceOption {
    pub name: OptionName,
    pub criteria: String,
    /// None: this option means "nothing to say".
    pub comment: Option<String>,
}

/// How Jev decides whether a rule applies to a hunk.
#[derive(Debug, Clone)]
pub enum Judge {
    /// Yes/no: comment when the probability of yes is high enough.
    Noul { question: String, comment: String },
    /// One of several options; each option carries its own comment.
    Choice {
        question: String,
        options: Vec<ChoiceOption>,
    },
}

/// Default bar for a noul rule: probability of yes.
pub const NOUL_THRESHOLD: f64 = 0.7;
/// Default bar for a choice rule: confidence in a commenting option.
pub const CHOICE_THRESHOLD: f64 = 0.6;

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: RuleId,
    pub severity: Severity,
    /// The bar Jev's score must reach; from the file, else the judge's default.
    pub threshold: f64,
    paths: GlobSet,
    /// Places the comment on the first matching added line; judges alone
    /// when no Jev key is configured.
    pub hint: Option<Regex>,
    /// Comment only where the hint matches an added line: the regex finds
    /// the spot, Jev confirms it and picks the fix.
    pub gate: bool,
    pub judge: Judge,
    pub also_guarded_by: Vec<String>,
    pub examples: Vec<String>,
}

impl Rule {
    pub fn applies_to(&self, path: &str) -> bool {
        self.paths.is_match(path)
    }
}

#[derive(Debug)]
pub struct RuleSet {
    rules: Vec<Rule>,
}

impl RuleSet {
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Every `*.toml` in `dir`, in file name order.
    pub fn load_dir(dir: &Path) -> Result<Self> {
        let mut files: Vec<_> = fs::read_dir(dir)
            .with_context(|| format!("reading rules directory {}", dir.display()))?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<_>>()?;
        files.retain(|p| p.extension().is_some_and(|e| e == "toml"));
        files.sort();
        let rules = files
            .iter()
            .map(|p| {
                let id = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .with_context(|| format!("{}: file name is not UTF-8", p.display()))?;
                let text = fs::read_to_string(p)?;
                parse_rule(id, &text).with_context(|| format!("in {}", p.display()))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { rules })
    }
}

// ---- TOML shape, validated into Rule ----

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRule {
    severity: Severity,
    paths: Vec<String>,
    #[serde(default)]
    hint: Option<String>,
    /// Set by calibration (`dify-review backtest`).
    #[serde(default)]
    threshold: Option<f64>,
    #[serde(default)]
    gate: bool,
    #[serde(default)]
    also_guarded_by: Vec<String>,
    /// For people deciding what to turn into a dify guard; unused here.
    #[serde(default)]
    #[allow(dead_code)]
    propose_guard: bool,
    #[serde(default)]
    examples: Vec<String>,
    #[serde(default)]
    noul: Option<RawNoul>,
    #[serde(default)]
    choice: Option<RawChoice>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNoul {
    question: String,
    comment: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawChoice {
    question: String,
    options: BTreeMap<String, RawOption>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOption {
    criteria: String,
    #[serde(default)]
    comment: Option<String>,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// One rule file; `id` is its file name without `.toml`.
pub fn parse_rule(id: &str, text: &str) -> Result<Rule> {
    ensure!(valid_id(id), "rule id {id:?} must be kebab-case");
    let r: RawRule = toml::from_str(text)?;
    let ctx = |what: &str| format!("rule {id}: {what}");
    ensure!(!r.paths.is_empty(), ctx("paths is empty"));
    let paths = r
        .paths
        .iter()
        .try_fold(GlobSetBuilder::new(), |mut b, p| {
            b.add(GlobBuilder::new(p).literal_separator(true).build()?);
            Ok::<_, globset::Error>(b)
        })
        .and_then(|b| b.build())
        .with_context(|| ctx("bad path glob"))?;
    let hint = r
        .hint
        .as_deref()
        .map(Regex::new)
        .transpose()
        .with_context(|| ctx("bad hint regex"))?;
    let judge = match (r.noul, r.choice) {
        (Some(n), None) => Judge::Noul {
            question: n.question.trim().to_owned(),
            comment: n.comment.trim().to_owned(),
        },
        (None, Some(c)) => {
            ensure!(
                c.options.len() >= 2,
                ctx("a choice needs at least two options")
            );
            ensure!(
                c.options.values().any(|o| o.comment.is_some()),
                ctx("a choice needs an option with a comment")
            );
            Judge::Choice {
                question: c.question.trim().to_owned(),
                options: c
                    .options
                    .into_iter()
                    .map(|(name, o)| ChoiceOption {
                        name: OptionName(name),
                        criteria: o.criteria.trim().to_owned(),
                        comment: o.comment.map(|c| c.trim().to_owned()),
                    })
                    .collect(),
            }
        }
        (Some(_), Some(_)) => bail!(ctx("has both [noul] and [choice]")),
        (None, None) => bail!(ctx("needs [noul] or [choice]")),
    };
    let threshold = r.threshold.unwrap_or(match judge {
        Judge::Noul { .. } => NOUL_THRESHOLD,
        Judge::Choice { .. } => CHOICE_THRESHOLD,
    });
    ensure!(
        threshold > 0.0 && threshold <= 1.0,
        ctx("threshold must be in (0, 1]")
    );
    ensure!(!r.gate || hint.is_some(), ctx("gate = true needs a hint"));
    Ok(Rule {
        id: RuleId(id.to_owned()),
        severity: r.severity,
        threshold,
        paths,
        hint,
        gate: r.gate,
        judge,
        also_guarded_by: r.also_guarded_by,
        examples: r.examples,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const NOUL: &str = r#"
severity = 'should'
paths = ['api/**/*.py']
hint = '#\s*type:\s*ignore'
examples = ['https://example.test/r1']

[noul]
question = '''
Does `patch` add a suppression?'''
comment = '''
Fix the type.'''
"#;

    pub(crate) const CHOICE: &str = r#"
severity = 'nit'
paths = ['api/tests/**/*.py']

[choice]
question = 'How?'

[choice.options.real]
criteria = 'a'
comment = 'Use the real class.'

[choice.options.none]
criteria = 'b'
"#;

    #[test]
    fn loads_and_matches_paths() {
        let noul = parse_rule("no-type-ignore", NOUL).unwrap();
        assert!(noul.applies_to("api/core/x.py"));
        assert!(!noul.applies_to("web/x.py"));
        assert!(matches!(&noul.judge, Judge::Noul { comment, .. } if comment == "Fix the type."));
        let choice = parse_rule("doubles", CHOICE).unwrap();
        assert!(choice.applies_to("api/tests/unit_tests/a/test_b.py"));
        assert!(!choice.applies_to("api/core/tests.py"));
        assert!(matches!(&choice.judge, Judge::Choice { options, .. }
            if options.len() == 2 && options.iter().any(|o| o.comment.is_none())));
    }

    #[test]
    fn rejects_bad_rules() {
        let head = "severity = 'nit'\npaths = ['**']\n";
        let noul = "\n[noul]\nquestion = 'q'\ncomment = 'c'\n";
        let cases = [
            (
                "both judges",
                format!(
                    "{head}{noul}\n[choice]\nquestion = 'q'\n[choice.options.a]\ncriteria = 'x'\ncomment = 'y'\n[choice.options.b]\ncriteria = 'z'\n"
                ),
            ),
            ("no judge", head.to_owned()),
            ("bad regex", format!("{head}hint = '('\n{noul}")),
            ("unknown field", format!("{head}colour = 'red'\n{noul}")),
            (
                "silent choice",
                format!(
                    "{head}\n[choice]\nquestion = 'q'\n[choice.options.a]\ncriteria = 'x'\n[choice.options.b]\ncriteria = 'z'\n"
                ),
            ),
            ("no paths", format!("severity = 'nit'\npaths = []\n{noul}")),
        ];
        for (name, text) in cases {
            assert!(parse_rule("r", &text).is_err(), "{name}");
        }
        assert!(parse_rule("Bad_Id", &format!("{head}{noul}")).is_err());
    }

    #[test]
    fn repository_rules_load() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rules");
        let set = RuleSet::load_dir(&dir).unwrap();
        assert!(set.rules().len() >= 10);
    }
}
