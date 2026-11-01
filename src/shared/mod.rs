pub mod paths;
pub mod process;

/// A sha as a person reads it.
///
/// Compared in full and shown short: nobody reads forty characters, and seven
/// is what every other tool prints. Above the layer that prints it because
/// three of them do.
pub fn short(sha: &str) -> String {
    sha.chars().take(7).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sha_is_shortened_to_what_a_reader_can_hold() {
        assert_eq!(short("a3f1e9c1234567890"), "a3f1e9c");
    }

    #[test]
    fn a_sha_shorter_than_the_cut_is_left_alone() {
        assert_eq!(short("abc"), "abc");
    }
}
