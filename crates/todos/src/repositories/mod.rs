//! Repository pattern implementation for data access
//!
//! This module provides repository implementations for different entities,
//! separating data access logic from business logic.
//!
//! # 架构设计
//!
//! 所有 Repository 都遵循统一的接口规范：
//! - `BaseRepository<T>`: 基础 CRUD 操作

pub mod base_repository;
pub mod item_label_repository;
pub mod item_repository;
pub mod label_repository;
pub mod section_repository;

pub use base_repository::*;
pub use item_label_repository::*;
pub use item_repository::*;
pub use label_repository::*;
pub use section_repository::*;
