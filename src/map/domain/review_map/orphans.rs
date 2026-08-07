//! Notes waiting for a decision after the code under them moved.
use super::*;

impl ReviewMap {
    pub fn take_orphan(&mut self, slug: &Slug, path: &str, range: LineRange) -> Result<Orphan> {
        let idx = self
            .orphans
            .iter()
            .position(|o| &o.block == slug && o.path == path && o.old_range == range)
            .ok_or_else(|| Error::NoSuchOrphan {
                slug: slug.to_string(),
                path: path.to_string(),
                range,
            })?;
        Ok(self.orphans.remove(idx))
    }
}
