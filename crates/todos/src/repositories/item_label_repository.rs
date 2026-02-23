//! Item-Label 关联表 Repository
//!
//! 提供 Item 和 Label 之间多对多关系的 CRUD 操作

use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set,
};

use crate::{
    entity::{
        ItemLabelModel, LabelModel,
        prelude::{ItemLabelEntity, LabelEntity},
    },
    error::TodoError,
};

/// Repository trait for Item-Label operations
#[async_trait::async_trait]
pub trait ItemLabelRepository {
    /// 为 Item 添加一个 Label
    async fn add_label_to_item(
        &self,
        item_id: &str,
        label_id: &str,
    ) -> Result<ItemLabelModel, TodoError>;

    /// 从 Item 移除一个 Label
    async fn remove_label_from_item(&self, item_id: &str, label_id: &str)
    -> Result<u64, TodoError>;

    /// 获取 Item 的所有 Labels
    async fn get_labels_by_item(&self, item_id: &str) -> Result<Vec<LabelModel>, TodoError>;

    /// 获取 Label 关联的所有 Items
    async fn get_items_by_label(&self, label_id: &str) -> Result<Vec<String>, TodoError>;

    /// 批量设置 Item 的 Labels（先删除旧关联，再添加新关联）
    async fn set_item_labels(&self, item_id: &str, label_ids: &[String]) -> Result<(), TodoError>;

    /// 删除 Item 的所有 Label 关联
    async fn remove_all_labels_from_item(&self, item_id: &str) -> Result<u64, TodoError>;

    /// 删除 Label 的所有 Item 关联
    async fn remove_all_items_from_label(&self, label_id: &str) -> Result<u64, TodoError>;

    /// 检查 Item 是否有某个 Label
    async fn has_label(&self, item_id: &str, label_id: &str) -> Result<bool, TodoError>;
}

/// Implementation of ItemLabelRepository
#[derive(Clone, Debug)]
pub struct ItemLabelRepositoryImpl {
    db: Arc<DatabaseConnection>,
}

impl ItemLabelRepositoryImpl {
    /// Create a new ItemLabelRepository
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl ItemLabelRepository for ItemLabelRepositoryImpl {
    /// 为 Item 添加一个 Label
    ///
    /// # 参数
    /// * `item_id` - Item ID
    /// * `label_id` - Label ID
    ///
    /// # 返回
    /// * `Ok(ItemLabelModel)` - 创建成功的关联记录
    /// * `Err(TodoError)` - 创建失败（如已存在）
    async fn add_label_to_item(
        &self,
        item_id: &str,
        label_id: &str,
    ) -> Result<ItemLabelModel, TodoError> {
        use crate::entity::item_labels::ActiveModel;

        let active_model = ActiveModel {
            item_id: Set(item_id.to_string()),
            label_id: Set(label_id.to_string()),
            ..Default::default()
        };

        active_model
            .insert(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(format!("Failed to add label to item: {}", e)))
    }

    /// 从 Item 移除一个 Label
    ///
    /// # 参数
    /// * `item_id` - Item ID
    /// * `label_id` - Label ID
    ///
    /// # 返回
    /// * `Ok(u64)` - 删除的记录数
    async fn remove_label_from_item(
        &self,
        item_id: &str,
        label_id: &str,
    ) -> Result<u64, TodoError> {
        use crate::entity::item_labels::Column;

        ItemLabelEntity::delete_many()
            .filter(Column::ItemId.eq(item_id))
            .filter(Column::LabelId.eq(label_id))
            .exec(&*self.db)
            .await
            .map(|res| res.rows_affected)
            .map_err(|e| {
                TodoError::DatabaseError(format!("Failed to remove label from item: {}", e))
            })
    }

    /// 获取 Item 的所有 Labels
    ///
    /// # 参数
    /// * `item_id` - Item ID
    ///
    /// # 返回
    /// * `Ok(Vec<LabelModel>)` - Label 列表
    async fn get_labels_by_item(&self, item_id: &str) -> Result<Vec<LabelModel>, TodoError> {
        use crate::entity::item_labels::Column as ItemLabelColumn;

        // 通过 JOIN 查询获取 Label 详情
        let label_ids: Vec<String> = ItemLabelEntity::find()
            .select_only()
            .column(ItemLabelColumn::LabelId)
            .filter(ItemLabelColumn::ItemId.eq(item_id))
            .into_tuple::<String>()
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(format!("Failed to get label IDs: {}", e)))?;

        if label_ids.is_empty() {
            return Ok(vec![]);
        }

        // 查询 Label 详情
        LabelEntity::find()
            .filter(crate::entity::labels::Column::Id.is_in(label_ids))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(format!("Failed to get labels: {}", e)))
    }

    /// 获取 Label 关联的所有 Items
    ///
    /// # 参数
    /// * `label_id` - Label ID
    ///
    /// # 返回
    /// * `Ok(Vec<String>)` - Item ID 列表
    async fn get_items_by_label(&self, label_id: &str) -> Result<Vec<String>, TodoError> {
        use crate::entity::item_labels::Column;

        ItemLabelEntity::find()
            .select_only()
            .column(Column::ItemId)
            .filter(Column::LabelId.eq(label_id))
            .into_tuple::<String>()
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(format!("Failed to get items by label: {}", e)))
    }

    /// 批量设置 Item 的 Labels
    ///
    /// 先删除该 Item 的所有旧 Label 关联，再添加新的关联
    ///
    /// # 参数
    /// * `item_id` - Item ID
    /// * `label_ids` - Label ID 列表
    async fn set_item_labels(&self, item_id: &str, label_ids: &[String]) -> Result<(), TodoError> {
        use crate::entity::item_labels::ActiveModel;

        tracing::info!(
            "ItemLabelRepository::set_item_labels START - item_id: {}, label_ids: {:?}",
            item_id,
            label_ids
        );

        // 1. 删除旧关联
        tracing::info!("Removing all old labels from item: {}", item_id);
        let deleted_count = self.remove_all_labels_from_item(item_id).await?;
        tracing::info!("Removed {} old labels from item: {}", deleted_count, item_id);

        // 2. 添加新关联
        tracing::info!("Adding {} new labels to item: {}", label_ids.len(), item_id);
        for label_id in label_ids {
            tracing::info!("Inserting label_id: {} for item_id: {}", label_id, item_id);
            let active_model = ActiveModel {
                item_id: Set(item_id.to_string()),
                label_id: Set(label_id.clone()),
                ..Default::default()
            };

            match active_model.insert(&*self.db).await {
                Ok(_) => tracing::info!("Successfully inserted label_id: {}", label_id),
                Err(e) => {
                    tracing::error!("Failed to insert label_id: {} - {:?}", label_id, e);
                    return Err(TodoError::DatabaseError(format!(
                        "Failed to set item labels: {}",
                        e
                    )));
                },
            }
        }

        tracing::info!("ItemLabelRepository::set_item_labels SUCCESS - item_id: {}", item_id);
        Ok(())
    }

    /// 删除 Item 的所有 Label 关联
    ///
    /// # 参数
    /// * `item_id` - Item ID
    ///
    /// # 返回
    /// * `Ok(u64)` - 删除的记录数
    async fn remove_all_labels_from_item(&self, item_id: &str) -> Result<u64, TodoError> {
        use crate::entity::item_labels::Column;

        tracing::info!("remove_all_labels_from_item: building query for item_id: {}", item_id);
        let query = ItemLabelEntity::delete_many().filter(Column::ItemId.eq(item_id));

        tracing::info!("remove_all_labels_from_item: executing query...");
        let start = std::time::Instant::now();
        let result = query.exec(&*self.db).await;
        let elapsed = start.elapsed();

        match result {
            Ok(res) => {
                tracing::info!(
                    "remove_all_labels_from_item: SUCCESS - deleted {} rows in {:?}",
                    res.rows_affected,
                    elapsed
                );
                Ok(res.rows_affected)
            },
            Err(e) => {
                tracing::error!("remove_all_labels_from_item: FAILED - {:?} in {:?}", e, elapsed);
                Err(TodoError::DatabaseError(format!(
                    "Failed to remove all labels from item: {}",
                    e
                )))
            },
        }
    }

    /// 删除 Label 的所有 Item 关联
    ///
    /// # 参数
    /// * `label_id` - Label ID
    ///
    /// # 返回
    /// * `Ok(u64)` - 删除的记录数
    async fn remove_all_items_from_label(&self, label_id: &str) -> Result<u64, TodoError> {
        use crate::entity::item_labels::Column;

        ItemLabelEntity::delete_many()
            .filter(Column::LabelId.eq(label_id))
            .exec(&*self.db)
            .await
            .map(|res| res.rows_affected)
            .map_err(|e| {
                TodoError::DatabaseError(format!("Failed to remove all items from label: {}", e))
            })
    }

    /// 检查 Item 是否有某个 Label
    ///
    /// # 参数
    /// * `item_id` - Item ID
    /// * `label_id` - Label ID
    ///
    /// # 返回
    /// * `Ok(true)` - Item 有该 Label
    /// * `Ok(false)` - Item 没有该 Label
    async fn has_label(&self, item_id: &str, label_id: &str) -> Result<bool, TodoError> {
        use crate::entity::item_labels::Column;

        let count = ItemLabelEntity::find()
            .filter(Column::ItemId.eq(item_id))
            .filter(Column::LabelId.eq(label_id))
            .count(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(format!("Failed to check label: {}", e)))?;

        Ok(count > 0)
    }
}
