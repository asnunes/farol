use crate::error::Result;

/// Branch identity matters even when two branches point at the same commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadState {
    pub reference: Option<String>,
    pub commit: String,
}

pub trait HeadSource: Send + Sync {
    fn read_head(&self) -> Result<HeadState>;
}
