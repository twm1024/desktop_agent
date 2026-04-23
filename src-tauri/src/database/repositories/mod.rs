// Copyright 2024 Desktop Agent Team
// Licensed under MIT License

//! Database repository layer

#![allow(dead_code)]
#![allow(unused_imports)]
pub mod skill_repository;
pub mod user_repository;
pub mod session_repository;
pub mod task_repository;
pub mod log_repository;

pub use skill_repository::{SkillRepository, SkillRecord};
pub use user_repository::UserRepository;
pub use session_repository::SessionRepository;
pub use task_repository::{TaskRepository, TaskRecord, TaskStatus};
pub use log_repository::LogRepository;
