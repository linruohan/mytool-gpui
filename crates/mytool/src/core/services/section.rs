use std::sync::Arc;

use sea_orm::DatabaseConnection;
use todos::{Store, entity::SectionModel, error::TodoError};

/// 加载所有的 sections
pub async fn load_sections(db: DatabaseConnection) -> Vec<SectionModel> {
    Store::new(db).await.unwrap().get_all_sections().await.unwrap_or_default()
}

/// 使用全局 Store 加载 sections（推荐）
pub async fn load_sections_with_store(store: Arc<Store>) -> Vec<SectionModel> {
    store.get_all_sections().await.unwrap_or_default()
}

/// 新增 section
pub async fn add_section(
    section: Arc<SectionModel>,
    db: DatabaseConnection,
) -> Result<SectionModel, TodoError> {
    Store::new(db).await?.insert_section(section.as_ref().clone()).await
}

/// 使用全局 Store Add Section（推荐）
pub async fn add_section_with_store(
    section: Arc<SectionModel>,
    store: Arc<Store>,
) -> Result<SectionModel, TodoError> {
    store.insert_section(section.as_ref().clone()).await
}

/// 修改 section
pub async fn mod_section(
    section: Arc<SectionModel>,
    db: DatabaseConnection,
) -> Result<SectionModel, TodoError> {
    Store::new(db).await?.update_section(section.as_ref().clone()).await
}

/// 使用全局 Store 修改 section（推荐）
pub async fn mod_section_with_store(
    section: Arc<SectionModel>,
    store: Arc<Store>,
) -> Result<SectionModel, TodoError> {
    store.update_section(section.as_ref().clone()).await
}

/// 删除 section
pub async fn del_section(
    section: Arc<SectionModel>,
    db: DatabaseConnection,
) -> Result<(), TodoError> {
    Store::new(db).await?.delete_section(&section.id).await
}

/// 使用全局 Store 删除 section（推荐）
pub async fn del_section_with_store(
    section: Arc<SectionModel>,
    store: Arc<Store>,
) -> Result<(), TodoError> {
    store.delete_section(&section.id).await
}
