pub mod paths;

/// Stands in for a commit id when the map covers uncommitted work.
///
/// Both layers need it — the diff layer to know it should read the working
/// tree, the map layer to name the version — so it belongs to neither.
pub const WORKING: &str = "working";

/// A sha as a person reads it.
///
/// Compared in full and shown short: nobody reads forty characters, and seven
/// is what every other tool prints. Beside `WORKING` because the marker is the
/// one thing it has to know not to cut, and above the layer that prints it
/// because three of them do.
pub fn short(sha: &str) -> String {
    if sha == WORKING {
        sha.to_string()
    } else {
        sha.chars().take(7).collect()
    }
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

    #[test]
    fn the_working_tree_marker_is_printed_whole() {
        // Cutting it to seven characters would turn a word into gibberish.
        assert_eq!(short(WORKING), WORKING);
    }
}
