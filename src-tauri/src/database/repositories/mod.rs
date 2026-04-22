// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Database repository layer

pub mod skill_repository;
pub mod user_repository;
pub mod session_repository;
pub mod task_repository;
pub mod log_repository;

pub use skill_repository::SkillRepository;
pub use user_repository::UserRepository;
pub use session_repository::SessionRepository;
pub use task_repository::TaskRepository;
pub use log_repository::LogRepository;
