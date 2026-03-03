//! Jarvis Daemon library — re-exports for integration tests.
//!
//! The daemon is primarily a binary (`jarvis-daemon`). This library target
//! exposes internal modules so that integration tests in `tests/` can
//! construct pipelines, mock LLM clients, and exercise the full pipeline
//! execution flow without needing to spin up the entire scheduler.

pub mod data_sources;
pub mod decision_engine;
pub mod executor;
pub mod notifications;
pub mod pipeline;
pub mod pipelines;
pub mod processor;
pub mod publisher;
pub mod runner;
pub mod scheduler;
pub mod scraper;
