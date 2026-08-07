//! Where a newly added block or file goes in the reading order.

use super::slug::Slug;

/// Where a newly added block or file goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Position {
    End,
    /// Ahead of the named block.
    Before(Slug),
    /// Behind the named block.
    After(Slug),
}
