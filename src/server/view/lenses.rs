//! A lens as the page draws it.

use serde::Serialize;

use crate::diff::domain::Side;
use crate::lenses::application::Lensed;
use crate::lenses::domain::{Freshness, LensTarget};
use crate::progress::domain::Progress;
use crate::shared::{WORKING, short};

/// One lens on one target, and what it said.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LensView {
    pub id: String,
    pub title: String,
    /// `written`, `outdated`, or `missing` when nothing was written yet.
    pub state: &'static str,
    pub text: String,
    /// The short commit it was written at, or `uncommitted work`.
    pub written_at: Option<String>,
    /// Files changed since it was written; zero when only its prompt did.
    pub changed: usize,
    /// The citations that can be followed, exactly as they appear in the text.
    pub citations: Vec<CitationView>,
    /// What it is written about, sent back to mark it seen.
    pub target: Option<LensTarget>,
    /// Marked as seen by the reader, for this very answer.
    pub seen: bool,
    /// Why the reader should see the file's code, when a file lens says so.
    pub open: Option<String>,
}

impl LensView {
    pub fn all(lensed: Vec<Lensed<'_>>, progress: &Progress) -> Vec<Self> {
        lensed.into_iter().map(|l| Self::of(l, progress)).collect()
    }

    fn of(lensed: Lensed<'_>, progress: &Progress) -> Self {
        let Some((answer, freshness)) = lensed.answer else {
            return Self {
                id: lensed.lens.id.to_string(),
                title: lensed.lens.title.clone(),
                state: "missing",
                text: String::new(),
                written_at: None,
                changed: 0,
                citations: Vec::new(),
                target: None,
                seen: false,
                open: None,
            };
        };
        let (state, changed) = match freshness {
            Freshness::Written => ("written", 0),
            Freshness::Outdated { changed } => ("outdated", changed),
        };
        Self {
            id: lensed.lens.id.to_string(),
            title: lensed.lens.title.clone(),
            state,
            text: answer.text.clone(),
            written_at: Some(if answer.written_at == WORKING {
                "uncommitted work".into()
            } else {
                short(&answer.written_at)
            }),
            changed,
            citations: answer
                .citations
                .iter()
                .filter(|c| c.linked)
                .map(|c| CitationView {
                    text: c.text.clone(),
                    path: c.path.clone(),
                    side: c.side,
                    from: c.from,
                    to: c.to,
                })
                .collect(),
            target: Some(answer.target.clone()),
            seen: progress.has_seen(&answer.seen_key(), &answer.fingerprint()),
            open: answer.open.clone(),
        }
    }
}

/// A place in the code the page can take the reader to.
#[derive(Debug, Serialize)]
pub struct CitationView {
    pub text: String,
    pub path: String,
    pub side: Side,
    pub from: u32,
    pub to: u32,
}
