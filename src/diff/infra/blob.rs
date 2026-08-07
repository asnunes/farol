//! Git's name for a file's contents, and which side of a diff it stands on.

/// A file's bytes together with git's own name for them.
///
/// The id is the blob's object id — read straight off the tree entry, not
/// computed. It is what viewed-state keys on, and what tells two versions of a
/// file apart without comparing them byte by byte.
pub(super) struct Blob {
    pub data: Vec<u8>,
    pub id: gix::ObjectId,
}

impl Blob {
    pub fn hash(&self) -> String {
        self.id.to_hex().to_string()
    }
}

/// Where one side of a diff comes from.
pub(super) enum Side<'a> {
    /// The file does not exist on this side — an addition or a deletion.
    Absent,
    Object(&'a Blob),
    /// Uncommitted work, which has no object to point at yet.
    Worktree,
}

impl Side<'_> {
    /// A null id means "no object": with a worktree root set for this side git
    /// reads the file from disk, and without one the side simply is not there.
    /// The empty blob would be wrong — it is an object, and a fresh repository
    /// has never stored it.
    pub(super) fn id(&self, hash: gix::hash::Kind) -> gix::ObjectId {
        match self {
            Side::Absent | Side::Worktree => gix::ObjectId::null(hash),
            Side::Object(blob) => blob.id,
        }
    }
}

/// What came back from asking git to diff a file.
pub(super) enum Diffed {
    /// git will not diff it: binary content, or `-diff` in `.gitattributes`.
    Untouchable,
    Text(super::text_diff::Hunks),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(content: &str) -> Blob {
        let data = content.as_bytes().to_vec();
        let id =
            gix::objs::compute_hash(gix::hash::Kind::Sha1, gix::object::Kind::Blob, &data).unwrap();
        Blob { data, id }
    }

    #[test]
    fn a_blob_is_named_the_way_git_names_it() {
        // `git hash-object` prints this for a file containing "hello\n".
        assert_eq!(
            blob("hello\n").hash(),
            "ce013625030ba8dba906f756967f9e9ca394464a"
        );
    }

    #[test]
    fn a_side_holding_an_object_hands_git_that_object() {
        let b = blob("hello\n");

        assert_eq!(Side::Object(&b).id(gix::hash::Kind::Sha1), b.id);
    }

    #[test]
    fn a_side_with_no_object_hands_git_the_null_id() {
        // Both mean "no object here": with a worktree root set git reads the
        // file from disk, and without one the side simply is not there. The
        // empty blob would be wrong — it is an object, and a fresh repository
        // has never stored it.
        let null = gix::ObjectId::null(gix::hash::Kind::Sha1);

        assert_eq!(Side::Absent.id(gix::hash::Kind::Sha1), null);
        assert_eq!(Side::Worktree.id(gix::hash::Kind::Sha1), null);
    }
}
