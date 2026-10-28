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

    #[error("lens '{id}' has nothing written here yet. Write it with farol lens write.")]
    NothingToKeep { id: String },

    #[error(
        "lens answers for '{written}' and '{asked}' would share a file. Write it again; if this repeats, report it."
    )]
    Collision { written: String, asked: String },

    #[error(
        "lens '{id}' has {lines} lines and may have at most {most}. A lens exists to save the reader time: cut everything that would not change whether they approve, and write again."
    )]
    TooLong {
        id: String,
        lines: usize,
        most: usize,
    },

    #[error(
        "lens '{id}' has a line over {most} characters: \"{line}…\". One line, one point: split it or cut it."
    )]
    LineTooLong {
        id: String,
        line: String,
        most: usize,
    },

    #[error(
        "lens '{id}' says {said} characters about a change of {change}: reading the code is quicker. Write an empty text, and the file shows its code."
    )]
    LongerThanTheChange {
        id: String,
        said: usize,
        change: usize,
    },

    #[error("only a file lens opens what it is about: drop --open for lens '{id}'.")]
    OpensOnlyFiles { id: String },

    #[error(
        "an empty answer says lens '{id}' does not apply here, so it cannot open the file: write what the reader will see, or drop --open."
    )]
    OpensWithNothing { id: String },
}
