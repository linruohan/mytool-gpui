use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::SectionModel, error::TodoError};

// ==================== 加载分区 ====================

/// 加载所有的 sections
#[deprecated(since = "2.0", note = "请使用 load_sections_with_store() 方法")]
pub async fn load_sections(db: DatabaseConnection) -> Vec<SectionModel> {
    match Store::new(db).await {
        Ok(store) => store.get_all_sections().await.unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// 使用全局 Store 加载 sections（推荐）
pub async fn load_sections_with_store(store: Arc<Store>) -> Vec<SectionModel> {
    store.get_all_sections().await.unwrap_or_default()
}

// ==================== 添加分区 ====================

/// 新增 section
#[deprecated(since = "2.0", note = "请使用 add_section_with_store() 方法")]
pub async fn add_section(
    section: Arc<SectionModel>,
    db: DatabaseConnection,
) -> Result<SectionModel, TodoError> {
    let store = Arc::new(Store::new(db).await?);
    add_section_with_store(section, store).await
}

/// 使用全局 Store Add Section（推荐）
pub async fn add_section_with_store(
    section: Arc<SectionModel>,
    store: Arc<Store>,
) -> Result<SectionModel, TodoError> {
    store.insert_section(section.as_ref().clone()).await
}

// ==================== 修改分区 ====================

/// 修改 section
#[deprecated(since = "2.0", note = "请使用 mod_section_with_store() 方法")]
pub async fn mod_section(
    section: Arc<SectionModel>,
    db: DatabaseConnection,
) -> Result<SectionModel, TodoError> {
    let store = Arc::new(Store::new(db).await?);
    mod_section_with_store(section, store).await
}

/// 使用全局 Store 修改 section（推荐）
pub async fn mod_section_with_store(
    section: Arc<SectionModel>,
    store: Arc<Store>,
) -> Result<SectionModel, TodoError> {
    store.update_section(section.as_ref().clone()).await
}

// ==================== 删除分区 ====================

/// 删除 section
#[deprecated(since = "2.0", note = "请使用 del_section_with_store() 方法")]
pub async fn del_section(
    section: Arc<SectionModel>,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    let store = Arc::new(Store::new(db).await?);
    del_section_with_store(section, store).await
}

/// 使用全局 Store 删除 section（推荐）
pub async fn del_section_with_store(
    section: Arc<SectionModel>,
    store: Arc<Store>,
) -> Result<(), TodoError> {
    store.delete_section(&section.id).await
}
