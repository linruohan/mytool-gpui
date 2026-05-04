//! Repository 模式标准化实现
//!
//! 提供统一的数据访问接口，遵循 DDD (领域驱动设计) 的 Repository 模式。
//! 所有实体仓库都应实现 BaseRepository trait，确保接口一致性。

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter};

use crate::error::TodoError;

/// 仓库操作结果类型别名
pub type RepositoryResult<T> = Result<T, TodoError>;

/// 分页查询参数
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    /// 页码（从 1 开始）
    pub page: u64,
    /// 每页数量
    pub per_page: u64,
}

impl Pagination {
    /// 创建新的分页参数
    pub fn new(page: u64, per_page: u64) -> Self {
        Self { page: page.max(1), per_page: per_page.max(1) }
    }

    /// 计算偏移量
    pub fn offset(&self) -> u64 {
        (self.page - 1) * self.per_page
    }

    /// 获取限制数量
    pub fn limit(&self) -> u64 {
        self.per_page
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self { page: 1, per_page: 20 }
    }
}

/// 分页查询结果
#[derive(Debug, Clone)]
pub struct PagedResult<T> {
    /// 数据列表
    pub items: Vec<T>,
    /// 总数量
    pub total: u64,
    /// 当前页码
    pub page: u64,
    /// 每页数量
    pub per_page: u64,
    /// 总页数
    pub total_pages: u64,
}

impl<T> PagedResult<T> {
    /// 创建新的分页结果
    pub fn new(items: Vec<T>, total: u64, page: u64, per_page: u64) -> Self {
        let total_pages = if per_page > 0 { (total + per_page - 1) / per_page } else { 0 };
        Self { items, total, page, per_page, total_pages }
    }

    /// 是否有下一页
    pub fn has_next(&self) -> bool {
        self.page < self.total_pages
    }

    /// 是否有上一页
    pub fn has_prev(&self) -> bool {
        self.page > 1
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// 基础 Repository trait - 定义统一的 CRUD 接口
///
/// 所有实体仓库都应实现此 trait，确保数据访问层的一致性。
/// 泛型参数 T 代表实体类型。
#[async_trait]
pub trait BaseRepository<T>: Send + Sync {
    /// 根据 ID 查找单个实体
    ///
    /// # 参数
    /// - `id`: 实体 ID
    ///
    /// # 返回
    /// - `Ok(Some(entity))`: 找到实体
    /// - `Ok(None)`: 未找到实体
    /// - `Err(e)`: 数据库错误
    async fn find_by_id(&self, id: &str) -> RepositoryResult<Option<T>>;

    /// 查找所有实体
    async fn find_all(&self) -> RepositoryResult<Vec<T>>;

    /// 插入单个实体
    async fn insert(&self, entity: &T) -> RepositoryResult<T>;

    /// 更新单个实体
    async fn update(&self, entity: &T) -> RepositoryResult<T>;

    /// 根据 ID 删除实体
    ///
    /// # 返回
    /// - `Ok(true)`: 删除成功
    /// - `Ok(false)`: 实体不存在
    async fn delete(&self, id: &str) -> RepositoryResult<bool>;

    /// 批量插入实体
    async fn batch_insert(&self, entities: &[T]) -> RepositoryResult<Vec<T>>;

    /// 批量删除实体
    async fn batch_delete(&self, ids: &[&str]) -> RepositoryResult<u64>;

    /// 统计实体总数
    async fn count(&self) -> RepositoryResult<u64>;

    /// 检查实体是否存在
    async fn exists(&self, id: &str) -> RepositoryResult<bool> {
        Ok(self.find_by_id(id).await?.is_some())
    }
}

/// 可分页查询的 Repository trait
#[async_trait]
pub trait PageableRepository<T>: BaseRepository<T> {
    /// 分页查询实体
    async fn find_paged(&self, pagination: Pagination) -> RepositoryResult<PagedResult<T>>;
}

/// 支持条件查询的 Repository trait
#[async_trait]
pub trait ConditionalRepository<T>: BaseRepository<T> {
    /// 根据条件查询实体
    async fn find_by_condition(&self, condition: Condition) -> RepositoryResult<Vec<T>>;

    /// 根据条件统计数量
    async fn count_by_condition(&self, condition: Condition) -> RepositoryResult<u64>;
}

/// 仓库工厂 trait - 用于创建仓库实例
pub trait RepositoryFactory: Send + Sync {
    /// 获取数据库连接
    fn database(&self) -> &Arc<DatabaseConnection>;
}

/// 仓库基础实现结构体
///
/// 提供通用的数据库操作封装，减少重复代码。
#[derive(Clone, Debug)]
pub struct RepositoryBase {
    db: Arc<DatabaseConnection>,
}

impl RepositoryBase {
    /// 创建新的仓库基础实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取数据库连接引用
    pub fn database(&self) -> &DatabaseConnection {
        &self.db
    }

    /// 获取数据库连接的 Arc 引用
    pub fn database_arc(&self) -> Arc<DatabaseConnection> {
        self.db.clone()
    }
}

/// 软删除支持 trait
#[async_trait]
pub trait SoftDeletableRepository<T>: BaseRepository<T> {
    /// 软删除实体（标记为已删除）
    async fn soft_delete(&self, id: &str) -> RepositoryResult<bool>;

    /// 恢复软删除的实体
    async fn restore(&self, id: &str) -> RepositoryResult<bool>;

    /// 查找所有未删除的实体
    async fn find_active(&self) -> RepositoryResult<Vec<T>>;

    /// 查找所有已删除的实体
    async fn find_deleted(&self) -> RepositoryResult<Vec<T>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination() {
        let pagination = Pagination::new(2, 10);
        assert_eq!(pagination.page, 2);
        assert_eq!(pagination.per_page, 10);
        assert_eq!(pagination.offset(), 10);
        assert_eq!(pagination.limit(), 10);
    }

    #[test]
    fn test_pagination_default() {
        let pagination = Pagination::default();
        assert_eq!(pagination.page, 1);
        assert_eq!(pagination.per_page, 20);
    }

    #[test]
    fn test_pagination_min_values() {
        let pagination = Pagination::new(0, 0);
        assert_eq!(pagination.page, 1);
        assert_eq!(pagination.per_page, 1);
    }

    #[test]
    fn test_paged_result() {
        let items = vec![1, 2, 3];
        let result = PagedResult::new(items, 100, 2, 10);
        assert_eq!(result.items.len(), 3);
        assert_eq!(result.total, 100);
        assert_eq!(result.page, 2);
        assert_eq!(result.per_page, 10);
        assert_eq!(result.total_pages, 10);
        assert!(result.has_prev());
        assert!(result.has_next());
    }

    #[test]
    fn test_paged_result_last_page() {
        let items = vec![1, 2, 3];
        let result = PagedResult::new(items, 23, 3, 10);
        assert_eq!(result.total_pages, 3);
        assert!(!result.has_next());
        assert!(result.has_prev());
    }
}
