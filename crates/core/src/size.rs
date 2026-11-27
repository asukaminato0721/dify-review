//! How large a pull request is, counted the way pyrefly does: its
//! CONTRIBUTING ("Splitting a Pull Request": over about 150 lines of
//! non-test code, split it) and its size labeler (added lines only, test
//! files, lock files and snapshots left out; `size/xl` past 300).

/// Added lines outside tests, lock files and snapshots: what a reviewer has
/// to read closely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReviewLines(pub u64);

impl std::fmt::Display for ReviewLines {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// pyrefly's rule of thumb for suggesting a split.
pub const SUGGEST_SPLIT: ReviewLines = ReviewLines(150);
/// pyrefly's `size/xl`: too large to review well.
pub const REQUIRE_SPLIT: ReviewLines = ReviewLines(300);

/// Whether a changed file's added lines count toward `ReviewLines`.
pub fn counts(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let in_dir = |dir: &str| path.split('/').any(|part| part == dir);
    let test = in_dir("tests")
        || in_dir("test")
        || in_dir("__tests__")
        || (name.starts_with("test_") && name.ends_with(".py"))
        || name.ends_with("_test.py")
        || name.contains(".test.")
        || name.contains(".spec.");
    let lock =
        name.ends_with(".lock") || name.ends_with("-lock.json") || name.ends_with("-lock.yaml");
    let snapshot = path.contains("snapshots");
    !(test || lock || snapshot)
}

/// What to say about a pull request's size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeTier {
    Fine,
    /// Over the suggestion: ask once, still review it.
    SuggestSplit,
    /// Over the limit: ask once to split, and do not judge it.
    RequireSplit,
}

impl SizeTier {
    pub fn of(lines: ReviewLines, suggest: ReviewLines, require: ReviewLines) -> Self {
        match (lines > require, lines > suggest) {
            (true, _) => SizeTier::RequireSplit,
            (false, true) => SizeTier::SuggestSplit,
            (false, false) => SizeTier::Fine,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_out_tests_locks_and_snapshots() {
        assert!(counts(
            "api/core/rag/index_processor/index_processor_base.py"
        ));
        assert!(counts("web/app/components/base/button.tsx"));
        assert!(!counts(
            "api/tests/unit_tests/core/test_provider_manager.py"
        ));
        assert!(!counts("api/core/test_utils.py"));
        assert!(!counts("web/app/__tests__/button.tsx"));
        assert!(!counts("web/app/button.spec.tsx"));
        assert!(!counts("api/uv.lock"));
        assert!(!counts("web/pnpm-lock.yaml"));
        assert!(!counts("web/__snapshots__/a.snap"));
        // Only a whole directory name marks tests, not a prefix of one.
        assert!(counts("api/core/testing_tools/runner.py"));
    }

    #[test]
    fn tiers_follow_the_thresholds() {
        let tier = |n| SizeTier::of(ReviewLines(n), SUGGEST_SPLIT, REQUIRE_SPLIT);
        assert_eq!(tier(150), SizeTier::Fine);
        assert_eq!(tier(151), SizeTier::SuggestSplit);
        assert_eq!(tier(300), SizeTier::SuggestSplit);
        assert_eq!(tier(301), SizeTier::RequireSplit);
    }
}
