//! GitHub API integration for Jarvis.
//!
//! This crate provides a client for interacting with the GitHub API,
//! including operations for issues, pull requests, repositories, and Git operations.

pub mod client;
pub mod errors;
pub mod git;
pub mod issues;
pub mod models;
pub mod pull_requests;
pub mod repositories;

pub use client::GitHubClient;
pub use errors::GitHubError;
