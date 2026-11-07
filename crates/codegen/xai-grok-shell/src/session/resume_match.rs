//! Final `/resume` match shared by the session list, the picker, and `grok --resume`.
//!
//! One rule, not the older matchers in sequence: exact id, then a separator-insensitive
//! title, then fuzzy words. A typed word matches the start of a word, and a single
//! character stays that literal character. Callers include other folders and sort the
//! current folder and its checkouts ahead of the rest.

/// Id prefixes shorter than this stay title queries, so one letter does not hit every UUID.
const MIN_ID_PREFIX: usize = 8;

/// How wide a resume listing reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeListScope {
    /// The requested directory only.
    Only,
    /// The directory and its sibling checkouts.
    Siblings,
    /// Every local folder.
    All,
}

/// Search, and an explicit all-folders list, include sessions outside the current directory.
/// An exact cwd scope stays exact even when a query is set.
pub fn lists_other_folders(scope: ResumeListScope, has_query: bool) -> bool {
    matches!(scope, ResumeListScope::All) || (has_query && !matches!(scope, ResumeListScope::Only))
}

/// `true` when `query` hits `id` or any of `texts` (title, summary).
/// An empty query matches everything, which is the unfiltered picker.
pub fn query_matches(id: &str, texts: &[&str], query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    if id_matches(id, query) {
        return true;
    }
    let q_words = normalized_words(query);
    if q_words.is_empty() {
        return false;
    }
    let q_stripped: String = q_words.concat();
    texts
        .iter()
        .any(|text| text_matches(&q_words, &q_stripped, text))
}

fn id_matches(id: &str, query: &str) -> bool {
    if id.is_empty() {
        return false;
    }
    let id_l = id.to_lowercase();
    let q_l = query.to_lowercase();
    id_l == q_l || (q_l.len() >= MIN_ID_PREFIX && id_l.starts_with(&q_l))
}

fn text_matches(q_words: &[String], q_stripped: &str, text: &str) -> bool {
    let words = normalized_words(text);
    if words.is_empty() {
        return false;
    }
    let stripped: String = words.concat();
    if stripped == q_stripped {
        return true;
    }
    q_words
        .iter()
        .all(|word| words.iter().any(|title_word| title_word.starts_with(word.as_str())))
}

/// Lowercase words split on anything that is not a letter or digit.
pub fn normalized_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                current.push(lower);
            }
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// `true` when the separator-stripped title equals the separator-stripped query.
pub fn separator_insensitive_title(title: &str, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return false;
    }
    let q_words = normalized_words(query);
    if q_words.is_empty() {
        return false;
    }
    let title_words = normalized_words(title);
    !title_words.is_empty() && title_words.concat() == q_words.concat()
}

/// 0 = current folder, 1 = a checkout of it, 2 = any other folder.
/// No current folder means no preference: every row is band 0 so time order is unchanged.
pub fn folder_band(
    session_cwd: &str,
    git_root: Option<&str>,
    source_workspace: Option<&str>,
    worktree_label: Option<&str>,
    current_cwd: Option<&str>,
) -> u8 {
    let Some(current) = current_cwd.map(normalize_path).filter(|path| !path.is_empty()) else {
        return 0;
    };
    if normalize_path(session_cwd) == current {
        return 0;
    }
    if source_workspace
        .map(normalize_path)
        .is_some_and(|path| path == current)
    {
        return 1;
    }
    if git_root
        .map(normalize_path)
        .is_some_and(|path| path == current)
        && worktree_label.is_some_and(|label| !label.trim().is_empty())
    {
        return 1;
    }
    2
}

fn normalize_path(path: &str) -> &str {
    path.trim().trim_end_matches(['/', '\\'])
}

/// Up to five titles that are not matches but share a 3-character word prefix with the query.
pub fn similar_titles<'a>(query: &str, titles: &'a [String]) -> Vec<&'a str> {
    let q_words = normalized_words(query);
    if q_words.iter().all(|word| word.chars().count() < 3) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for title in titles {
        if query_matches("", &[title.as_str()], query) {
            continue;
        }
        let words = normalized_words(title);
        let close = q_words.iter().any(|query_word| {
            words
                .iter()
                .any(|title_word| shared_prefix(query_word, title_word) >= 3)
        });
        if close {
            out.push(title.as_str());
        }
        if out.len() == 5 {
            break;
        }
    }
    out
}

fn shared_prefix(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(a, b)| a == b)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything() {
        assert!(query_matches("abc", &["nope"], "  "));
    }

    #[test]
    fn exact_id_and_long_prefix_match_but_a_short_prefix_does_not() {
        let id = "019f870d-6976-7d73-a12a-52e9d4aebcd4";
        assert!(query_matches(id, &["unrelated"], id));
        assert!(query_matches(id, &["unrelated"], "019f870d"));
        assert!(!query_matches(id, &["unrelated"], "019"));
    }

    #[test]
    fn separator_insensitive_title_ignores_hyphens_underscores_and_case() {
        assert!(separator_insensitive_title(
            "Fix login bug",
            "fix-login-bug"
        ));
        assert!(separator_insensitive_title(
            "Fix login bug",
            "fix_login_bug"
        ));
        assert!(query_matches("", &["Fix login bug"], "fix-login-bug"));
        assert!(!separator_insensitive_title("Fix login bug", "fix-login"));
    }

    #[test]
    fn fuzzy_words_match_in_any_order_at_word_starts() {
        assert!(query_matches(
            "",
            &["Fix login bug"],
            "bug fix"
        ));
        assert!(query_matches("", &["Fix login bug"], "log"));
        assert!(
            !query_matches("", &["Blog prefix cleanup"], "log fix"),
            "log is inside Blog, not at the start of a word"
        );
        assert!(!query_matches("", &["rust-check"], "rc"));
    }

    #[test]
    fn single_character_stays_literal() {
        assert!(query_matches("", &["Alpha release"], "a"));
        assert!(!query_matches("", &["Beta release"], "a"));
        assert!(!query_matches("abcd-1234", &["Beta"], "a"));
    }

    #[test]
    fn other_folders_are_included_for_search_and_an_all_folders_list() {
        assert!(lists_other_folders(ResumeListScope::All, false));
        assert!(lists_other_folders(ResumeListScope::Siblings, true));
        assert!(!lists_other_folders(ResumeListScope::Siblings, false));
        assert!(!lists_other_folders(ResumeListScope::Only, true));
    }

    #[test]
    fn current_folder_and_its_checkouts_sort_ahead_of_other_folders() {
        assert_eq!(
            folder_band("/repo/", None, None, None, Some("/repo")),
            0
        );
        assert_eq!(
            folder_band(
                "/wt/feat",
                Some("/repo"),
                None,
                Some("feat"),
                Some("/repo")
            ),
            1
        );
        assert_eq!(
            folder_band(
                "/wt/feat",
                None,
                Some("/repo"),
                Some("feat"),
                Some("/repo")
            ),
            1
        );
        assert_eq!(
            folder_band("/elsewhere", Some("/elsewhere"), None, None, Some("/repo")),
            2
        );
        assert_eq!(folder_band("/elsewhere", None, None, None, None), 0);
    }

    #[test]
    fn similar_titles_share_a_word_prefix_and_skip_real_matches() {
        let titles = vec![
            "Fix login bug".to_string(),
            "Fixture cleanup".to_string(),
            "Blog prefix cleanup".to_string(),
        ];
        let similar = similar_titles("fixx", &titles);
        assert_eq!(similar, vec!["Fix login bug", "Fixture cleanup"]);
        assert!(similar_titles("zzz", &titles).is_empty());
    }
}
