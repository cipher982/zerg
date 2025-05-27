//! Sub-reducers that will eventually replace the monolithic `update.rs`.
//!
//! Each domain (chat, canvas, dashboard, …) lives in its own module.  The
//! root `update.rs` delegates to them while we migrate gradually so we can
//! keep the code compiling at every step.

pub mod chat;
