//! Reading progress: the store that keeps it, a **service**, and the use cases
//! over it, one per file.

mod store;
mod use_case;

pub use store::*;
pub use use_case::*;
