// SQL Server integration module
//
// Provides database connectivity and repository pattern implementation
// for persistent storage of users, conversations, messages, and more.

mod database;
mod repository;
mod models;
mod user_repository;
mod conversation_repository;
mod migrations;

pub use database::Database;
pub use repository::Repository;
pub use models::{User, Conversation, Message};
pub use user_repository::UserRepository;
pub use conversation_repository::ConversationRepository;
pub use migrations::Migrator;
