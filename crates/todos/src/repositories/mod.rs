//! Repository pattern implementation for data access
//!
//! This module provides repository implementations for different entities,
//! separating data access logic from business logic.

pub mod item_repository;
pub mod label_repository;
pub mod project_repository;
pub mod reminder_repository;
pub mod section_repository;

pub use item_repository::*;
pub use label_repository::*;
pub use project_repository::*;
pub use reminder_repository::*;
pub use section_repository::*;
