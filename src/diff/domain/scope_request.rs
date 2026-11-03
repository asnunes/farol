/// What the caller asked to review, before git resolves it: the refs as typed,
/// and whether to diff them directly rather than from their merge base.
///
/// In the domain because the server and the CLI both carry it, and neither
/// should learn of it from the git adapter that happens to resolve it.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ScopeRequest {
    pub base: Option<String>,
    pub head: Option<String>,
    pub direct: bool,
}
