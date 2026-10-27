use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::lenses::domain::{LensAnswer, LensAnswerRepository, LensId, LensTarget};
use crate::shared::paths::write_json;

/// One JSON file per lens and target, under the branch's store.
///
/// One per target, not one per lens: two answers written one after the other
/// then never read and rewrite the same file, and a file that cannot be read
/// costs one answer instead of a lens's worth.
pub struct JsonLensAnswers {
    directory: PathBuf,
}

impl JsonLensAnswers {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    fn path(&self, lens: &LensId, target: &LensTarget) -> PathBuf {
        self.directory.join(lens.as_str()).join(target.file_name())
    }
}

impl LensAnswerRepository for JsonLensAnswers {
    fn all(&self) -> Result<Vec<LensAnswer>> {
        let mut out = Vec::new();
        let Ok(lenses) = std::fs::read_dir(&self.directory) else {
            return Ok(out);
        };
        for lens in lenses.flatten() {
            let root = lens.path();
            let mut files = vec![root.join("pr.json")];
            for folder in ["block", "file"] {
                if let Ok(entries) = std::fs::read_dir(root.join(folder)) {
                    files.extend(entries.flatten().map(|e| e.path()));
                }
            }
            out.extend(files.iter().filter_map(|path| read(path)));
        }
        out.sort_by(|a, b| a.lens.cmp(&b.lens));
        Ok(out)
    }

    fn find(&self, lens: &LensId, target: &LensTarget) -> Result<Option<LensAnswer>> {
        Ok(read(&self.path(lens, target)))
    }

    fn write(&self, answer: &LensAnswer) -> Result<()> {
        let path = self.path(&answer.lens, &answer.target);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_json(&path, answer)
    }
}

/// Unreadable is absent, and said out loud: stored state is not migrated, and
/// an answer that no longer parses is written again on the next map update.
fn read(path: &Path) -> Option<LensAnswer> {
    if path.extension().is_none_or(|e| e != "json") {
        return None;
    }
    let body = std::fs::read(path).ok()?;
    match serde_json::from_slice(&body) {
        Ok(answer) => Some(answer),
        Err(error) => {
            eprintln!(
                "warning: discarding unreadable lens answer at {}: {error}",
                path.display()
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lenses::domain::Covered;

    fn answer(lens: &str, target: LensTarget) -> LensAnswer {
        LensAnswer {
            lens: LensId::parse(lens).unwrap(),
            target,
            text: "said".into(),
            written_at: "abc".into(),
            covers: vec![Covered {
                path: "a.rs".into(),
                blob: "1".into(),
            }],
            prompt_hash: "p".into(),
            read: vec![],
            citations: vec![],
        }
    }

    #[test]
    fn answers_written_for_every_kind_of_target_are_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonLensAnswers::new(dir.path().join("lenses"));
        let file = LensTarget::File {
            path: "src/deep/a.rs".into(),
        };

        store.write(&answer("overview", LensTarget::Pr)).unwrap();
        store.write(&answer("tests", file.clone())).unwrap();

        assert_eq!(store.all().unwrap().len(), 2);
        assert_eq!(
            store
                .find(&LensId::parse("tests").unwrap(), &file)
                .unwrap()
                .unwrap()
                .target,
            file
        );
    }

    #[test]
    fn an_unreadable_answer_is_left_out_rather_than_failing_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonLensAnswers::new(dir.path().join("lenses"));
        store.write(&answer("overview", LensTarget::Pr)).unwrap();
        std::fs::create_dir_all(dir.path().join("lenses/risk")).unwrap();
        std::fs::write(dir.path().join("lenses/risk/pr.json"), "{not json").unwrap();

        assert_eq!(store.all().unwrap().len(), 1);
    }
}
