//! Talking to the LLM session that implemented the branch.
//!
//! Two layers rather than four: nothing here is a use case. What gets sent and
//! when is decided by the part that owns the subject — a comment is sent by
//! `comments` — and this part only knows how to reach a harness.

pub mod domain;
pub mod infra;
