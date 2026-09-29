//! Echo - AI Electronic Pet
//! Library entry point for Tauri commands

pub mod window;
pub mod db;
pub mod chat;
pub mod embedding;
pub mod vector;
pub mod emotion;
pub mod evolution;
pub mod scheduler;
pub mod backup;
pub mod onboarding;

use tauri::command;

// Re-export commonly used types
pub use db::{DbManager, DbError};
pub use chat::{ChatEngine, ChatConfig, ChatChunk, ChatError, QuotaTracker};
pub use embedding::EmbeddingService;
pub use vector::VectorEngine;
pub use emotion::HybridAnalyzer;
pub use evolution::EvolutionEngine;
pub use scheduler::Scheduler;
pub use backup::BackupManager;
pub use onboarding::OnboardingManager;