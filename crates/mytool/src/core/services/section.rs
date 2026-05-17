use std::sync::Arc;

use todos::{Store, entity::SectionModel, error::TodoError};

// ==================== 加载分区 ====================

/// 使用全局 Store 加载 sections（推荐）
pub async fn load_sections_with_store(store: Arc<Store>) -> Result<Vec<SectionModel>, TodoError> {
    store.get_all_sections().await
}

// ==================== 添加分区 ====================

/// 新增 section（推荐）
pub async fn add_section_with_store(
    section: Arc<SectionModel>,
    store: Arc<Store>,
) -> Result<SectionModel, TodoError> {
    store.insert_section(section.as_ref().clone()).await
}

// ==================== 修改分区 ====================

/// 修改 section（推荐）
pub async fn mod_section_with_store(
    section: Arc<SectionModel>,
    store: Arc<Store>,
) -> Result<SectionModel, TodoError> {
    store.update_section(section.as_ref().clone()).await
}

// ==================== 删除分区 ====================

/// 删除 section（推荐）
pub async fn del_section_with_store(
    section: Arc<SectionModel>,
    store: Arc<Store>,
) -> Result<(), TodoError> {
    store.delete_section(&section.id).await
}
