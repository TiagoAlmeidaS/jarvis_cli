//! Shared types and database layer for the Jarvis daemon automation system.
//!
//! This crate provides:
//! - Data models for pipelines, jobs, content, sources, and metrics
//! - SQLite database operations (CRUD)
//! - Schedule parsing utilities
//!
//! It is used by both the `jarvis-daemon` binary and the `jarvis-cli`
//! daemon subcommands to ensure consistent data access.

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub mod db;
pub mod models;
pub mod schedule;

pub use db::DaemonDb;
pub use models::*;
pub use schedule::CronSchedule;
