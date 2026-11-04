//! The held-out test of the replay runner: copied to `tests/` after the run, never shown to the agent.

use replay_fixture::{word_count, word_frequencies};

fn pairs(list: &[(&str, usize)]) -> Vec<(String, usize)> {
    list.iter().map(|(w, n)| ((*w).to_string(), *n)).collect()
}

#[test]
fn word_count_is_unchanged() {
    assert_eq!(word_count("a b\tc"), 3);
}

#[test]
fn words_are_lower_cased_alphanumeric_runs() {
    assert_eq!(
        word_frequencies("The cat; the CAT! don't"),
        pairs(&[("cat", 2), ("the", 2), ("don", 1), ("t", 1)])
    );
}

#[test]
fn sorted_by_count_then_word() {
    assert_eq!(
        word_frequencies("b a b c 2 2 2"),
        pairs(&[("2", 3), ("b", 2), ("a", 1), ("c", 1)])
    );
}

#[test]
fn no_word_gives_nothing() {
    assert!(word_frequencies(" ,;! ").is_empty());
}
