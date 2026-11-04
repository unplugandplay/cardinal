//! Text statistics.

/// The number of whitespace-separated words of `text`.
#[must_use]
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    // spec: TXT-1 (internal: a unit test of the library, which has no binary)
    #[test]
    fn counts_whitespace_separated_words() {
        assert_eq!(word_count(" one  two\nthree "), 3);
    }
}
