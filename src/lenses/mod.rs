//! Lenses: what the session says about the change, so the reviewer can learn
//! it before reading the code.
//!
//! A lens is a title and a prompt in one of three groups: the pull request as
//! a whole, each block, or each file the prompt applies to. Its definition is
//! the user's, across repositories; its answers are written by the session
//! that built the branch, beside the map but not in it, because a map explains
//! and some lenses judge.

pub mod application;
pub mod domain;
pub mod infra;
pub mod presentation;
