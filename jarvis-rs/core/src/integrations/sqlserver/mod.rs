// SQL Server integration module
//
// Provides database connectivity and repository pattern implementation
// for persistent storage of users, conversations, messages, and more.

mod conversation_repository;
mod database;
mod migrations;
mod models;
mod repository;
mod user_repository;

pub use conversation_repository::ConversationRepository;
pub use database::Database;
pub use migrations::Migrator;
pub use models::Conversation;
pub use models::Message;
pub use models::User;
pub use repository::Repository;
pub use user_repository::UserRepository;
