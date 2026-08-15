//! The map, split by what each part changes for.
//!
//! `versions` decides which stored map applies; `derivation` produces the one
//! for the commit we are on; `editing` changes it; `reconciler` moves notes
//! when the code under them moved, and `bundle` is the shape a map takes to
//! leave the machine. All of them are **services** — dependencies of the use
//! cases, never called by a transport.

mod bundle;
mod derivation;
mod editing;
mod reconciler;
mod use_case;
mod versions;

pub use bundle::*;
pub use derivation::*;
pub use editing::*;
pub use reconciler::*;
pub use use_case::*;
pub use versions::*;
