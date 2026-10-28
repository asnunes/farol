/// What the lens part refuses. Each message says what to do, because most of
/// them are read by the session, which corrects itself from the message.
#[derive(Debug, thiserror::Error)]
pub enum LensError {
    #[error("unknown lens '{id}'. The lenses are: {}. Run farol lens list to see what each one asks.", known.join(", "))]
    UnknownLens { id: String, known: Vec<String> },

    #[error(
        "'{0}' is not a lens id: use lowercase letters, digits and dashes, as farol lens list shows them."
    )]
    BadId(String),

    #[error(
        "unknown group '{0}'. A lens is written for the pr, for each block, or for each file: use pr, block or file."
    )]
    UnknownGroup(String),

    #[error("a lens needs a title.")]
    NoTitle,

    #[error("a lens needs a prompt: it is what the session follows when it writes it.")]
    NoPrompt,

    #[error("lens '{id}' is written {wants}. {hint}")]
    WrongTarget {
        id: String,
        wants: &'static str,
        hint: &'static str,
    },

    #[error(
        "`{citation}` points past the end of {path}, which has {lines} lines. Cite lines that exist, or mark a removed line with (old)."
    )]
    PastTheEnd {
        citation: String,
        path: String,
        lines: u32,
    },

    #[error("lens '{id}' has nothing written here to keep. Write it with farol lens write.")]
    NothingToKeep { id: String },

    #[error(
        "lens answers for '{written}' and '{asked}' would share a file. Write it again; if this repeats, report it."
    )]
    Collision { written: String, asked: String },
}
