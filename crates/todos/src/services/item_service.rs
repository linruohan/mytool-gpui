//! Item service for business logic
//!
//! This module provides business logic for Item operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set, TransactionTrait, prelude::Expr,
};

use crate::{
    entity::{ItemActiveModel, ItemModel, items, prelude::*},
    error::TodoError,
    repositories::{
        BaseRepository, ItemLabelRepository, ItemLabelRepositoryImpl, ItemQueryRepository,
        ItemRepositoryImpl,
    },
    services::LabelService,
    utils::retry_with_context,
};

/// Service for Item business operations
#[derive(Clone, Debug)]
pub struct ItemService {
    db: Arc<DatabaseConnection>,
    label_service: Arc<LabelService>,
    item_repo: ItemRepositoryImpl,
    item_label_repo: ItemLabelRepositoryImpl,
}

impl ItemService {
    /// Create a new ItemService
    pub fn new(db: Arc<DatabaseConnection>, label_service: Arc<LabelService>) -> Self {
        let item_repo = ItemRepositoryImpl::new(db.clone());
        let item_label_repo = ItemLabelRepositoryImpl::new(db.clone());
        Self { db, label_service, item_repo, item_label_repo }
    }

    /// Get an item by ID
    pub async fn get_item(&self, id: &str) -> Option<ItemModel> {
        BaseRepository::find_by_id(&self.item_repo, id).await.ok().flatten()
    }

    /// Insert a new item
    pub async fn insert_item(
        &self,
        item: ItemModel,
        _insert: bool,
    ) -> Result<ItemModel, TodoError> {
        tracing::info!(
            "📝 [ItemService::insert_item] 开始插入任务, content='{}', project_id={:?}",
            item.content,
            item.project_id
        );

        let start = std::time::Instant::now();

        // 将 Model 转为 ActiveModel（Sea-ORM 的可写模型）
        let active_model: ItemActiveModel = item.into();

        // 执行 INSERT（SQLite 自动提交事务）
        let item_model = match active_model.insert(&*self.db).await {
            Ok(model) => model,
            Err(e) => {
                return Err(TodoError::DatabaseError(format!("INSERT 失败: {}", e)));
            },
        };

        tracing::info!(
            "✅ [ItemService::insert_item] INSERT 成功! id={}, 耗时={}ms",
            item_model.id,
            start.elapsed().as_millis()
        );

        Ok(item_model)
    }

    /// 更新任务项（核心方法）
    ///
    /// 将任务更新到数据库，支持重试机制
    pub async fn update_item(
        &self,
        item: ItemModel,
        _update_id: &str,
    ) -> Result<ItemModel, TodoError> {
        let item_id = item.id.clone();
        tracing::info!(
            "ItemService::update_item - id: {}, priority: {:?}, content: '{}'",
            item_id,
            item.priority,
            item.content
        );

        let now = chrono::Utc::now().naive_utc();

        let rows_affected = self.execute_item_update(&item, now).await?;
        if rows_affected == 0 {
            return Err(TodoError::not_found("Item").with_entity("Item", &item_id));
        }

        let mut updated_item = item;
        updated_item.updated_at = now;

        tracing::info!(
            "✅ 更新成功 - id: {}, content: '{}', priority: {:?}",
            updated_item.id,
            updated_item.content,
            updated_item.priority
        );

        Ok(updated_item)
    }

    /// 批量更新任务（单事务）
    pub async fn batch_update_items(
        &self,
        items: Vec<ItemModel>,
    ) -> Result<Vec<ItemModel>, TodoError> {
        if items.is_empty() {
            return Ok(vec![]);
        }

        let now = chrono::Utc::now().naive_utc();
        let db = self.db.clone();
        let items_to_update = items;

        let updated_items = db
            .transaction::<_, Vec<ItemModel>, TodoError>(|txn| {
                let items = items_to_update.clone();
                Box::pin(async move {
                    let mut results = Vec::with_capacity(items.len());
                    for item in items {
                        let item_id = item.id.clone();
                        let rows_affected = Self::update_item_fields_in_conn(txn, &item, now).await?;
                        if rows_affected == 0 {
                            return Err(TodoError::not_found("Item").with_entity("Item", &item_id));
                        }
                        let mut updated = item;
                        updated.updated_at = now;
                        results.push(updated);
                    }
                    Ok(results)
                })
            })
            .await
            .map_err(|e| match e {
                sea_orm::TransactionError::Connection(db_err) => TodoError::from(db_err),
                sea_orm::TransactionError::Transaction(err) => err,
            })?;

        Ok(updated_items)
    }

    /// 执行数据库更新操作，返回影响行数
    async fn execute_item_update(
        &self,
        item: &ItemModel,
        now: chrono::NaiveDateTime,
    ) -> Result<u64, TodoError> {
        let item_id = item.id.clone();
        let db = self.db.clone();
        let item_clone = item.clone();

        retry_with_context("execute_item_update", "Item", &item_id, || {
            let db = db.clone();
            let item = item_clone.clone();

            Box::pin(async move { Self::update_item_fields_in_conn(&*db, &item, now).await })
        })
        .await
    }

    /// 更新任务字段到数据库
    async fn update_item_fields_in_conn<C: ConnectionTrait>(
        conn: &C,
        item: &ItemModel,
        now: chrono::NaiveDateTime,
    ) -> Result<u64, TodoError> {
        let result = items::Entity::update_many()
            .col_expr(items::Column::Content, Expr::value(item.content.clone()))
            .col_expr(items::Column::Description, Expr::value(item.description.clone()))
            .col_expr(items::Column::Due, Expr::value(item.due.clone()))
            .col_expr(items::Column::UpdatedAt, Expr::value(now))
            .col_expr(items::Column::SectionId, Expr::value(item.section_id.clone()))
            .col_expr(items::Column::ProjectId, Expr::value(item.project_id.clone()))
            .col_expr(items::Column::ParentId, Expr::value(item.parent_id.clone()))
            .col_expr(items::Column::Priority, Expr::value(item.priority))
            .col_expr(items::Column::ChildOrder, Expr::value(item.child_order))
            .col_expr(items::Column::DayOrder, Expr::value(item.day_order))
            .col_expr(items::Column::Checked, Expr::value(item.checked))
            .col_expr(items::Column::IsDeleted, Expr::value(item.is_deleted))
            .col_expr(items::Column::Collapsed, Expr::value(item.collapsed))
            .col_expr(items::Column::Pinned, Expr::value(item.pinned))
            .col_expr(items::Column::Labels, Expr::value(item.labels.clone()))
            .col_expr(items::Column::ExtraData, Expr::value(item.extra_data.clone()))
            .col_expr(items::Column::ItemType, Expr::value(item.item_type.clone()))
            .filter(items::Column::Id.eq(item.id.clone()))
            .exec(conn)
            .await;

        match &result {
            Ok(res) => {
                tracing::info!("✅ update_many 成功, 影响行数: {}", res.rows_affected);
            },
            Err(e) => {
                tracing::error!("❌ update_many 失败 for item {}: {:?}", item.id, e);
            },
        }

        result.map(|res| res.rows_affected).map_err(TodoError::from)
    }

    /// Delete an item and its children
    ///
    /// 同时删除 item_labels 关联表中的记录（通过数据库级联删除）
    pub async fn delete_item(&self, item_id: &str) -> Result<(), TodoError> {
        let ids = self.collect_descendant_ids(item_id).await?;
        self.delete_items_by_ids(ids).await?;
        Ok(())
    }

    /// 收集任务及其所有子任务的 ID（按层批量查询，避免 N+1 逐条删除）
    pub(crate) async fn collect_descendant_ids(&self, root_id: &str) -> Result<Vec<String>, TodoError> {
        let mut result = vec![root_id.to_string()];
        let mut parents_to_search = vec![root_id.to_string()];

        while !parents_to_search.is_empty() {
            let batch = std::mem::take(&mut parents_to_search);
            let children = items::Entity::find()
                .filter(items::Column::ParentId.is_in(batch))
                .all(&*self.db)
                .await?;

            for child in children {
                result.push(child.id.clone());
                parents_to_search.push(child.id);
            }
        }

        Ok(result)
    }

    /// 批量删除任务（item_labels/reminders/attachments 由 FK CASCADE 处理）
    pub(crate) async fn delete_items_by_ids(&self, ids: Vec<String>) -> Result<(), TodoError> {
        if ids.is_empty() {
            return Ok(());
        }

        items::Entity::delete_many()
            .filter(items::Column::Id.is_in(ids))
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    /// Update item pin status
    pub async fn update_item_pin(&self, item_id: &str, pinned: bool) -> Result<(), TodoError> {
        let item = self
            .get_item(item_id)
            .await
            .ok_or_else(|| TodoError::not_found("Item").with_entity("Item", item_id))?;

        ItemEntity::update(ItemActiveModel { pinned: Set(pinned), ..item.into() })
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    /// Complete/uncomplete an item
    pub async fn complete_item(
        &self,
        item_id: &str,
        checked: bool,
        complete_subitems: bool,
    ) -> Result<(), TodoError> {
        let active_model = ItemActiveModel {
            id: Set(item_id.to_string()),
            checked: Set(checked),
            completed_at: Set(if checked { Some(chrono::Utc::now().naive_utc()) } else { None }),
            ..ItemEntity::find_by_id(item_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| TodoError::not_found("Item").with_entity("Item", item_id))?
                .into()
        };
        let item_model = active_model.update(&*self.db).await?;

        if complete_subitems {
            let subitems = ItemQueryRepository::find_by_parent(&self.item_repo, item_id).await?;
            if !subitems.is_empty() {
                let checked_value = item_model.checked;
                let completed_at_value =
                    if checked_value { Some(chrono::Utc::now().naive_utc()) } else { None };

                let sub_ids: Vec<String> = subitems.into_iter().map(|i| i.id).collect();
                let now = chrono::Utc::now().naive_utc();

                crate::entity::items::Entity::update_many()
                    .col_expr(items::Column::Checked, Expr::value(checked_value))
                    .col_expr(items::Column::CompletedAt, Expr::value(completed_at_value))
                    .col_expr(items::Column::UpdatedAt, Expr::value(now))
                    .filter(items::Column::Id.is_in(sub_ids))
                    .exec(&*self.db)
                    .await?;
            }
        }

        Ok(())
    }

    // ==================== Additional Business Logic Methods ====================

    /// Get all items in a project
    pub async fn get_items_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<ItemModel>, TodoError> {
        let items = ItemEntity::find()
            .filter(items::Column::ProjectId.eq(project_id))
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// Get all items (including completed and incomplete)
    ///
    /// 🚀 关键修复：使用批量加载 labels，避免 N+1 查询问题
    /// 原来：每个 item 都会触发一次 get_labels_by_item 查询
    /// 现在：只触发一次 get_all_item_labels 查询，然后将结果填充到 items
    pub async fn get_all_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        let items = ItemEntity::find().all(&*self.db).await?;

        tracing::info!("get_all_items: loaded {} items from database", items.len());

        // 🚀 优化：批量加载所有 item-labels 关联，避免 N+1 查询
        let all_item_labels = self.item_label_repo.get_all_item_labels().await?;

        let mut result = Vec::with_capacity(items.len());
        for mut item in items {
            tracing::debug!("get_all_items: item {} has due: {:?}", item.id, item.due);

            // 从批量加载的结果中获取该 item 的 labels
            if let Some(label_ids) = all_item_labels.get(&item.id) {
                item.labels = Some(label_ids.join(";"));
            } else {
                item.labels = None;
            }
            result.push(item);
        }

        Ok(result)
    }

    /// Add label to item
    ///
    /// 使用 item_labels 关联表维护 Item 和 Label 的关系
    pub async fn add_label_to_item(
        &self,
        item_id: &str,
        label_name: &str,
    ) -> Result<(), TodoError> {
        let label = self.label_service.get_or_create_label(label_name, item_id).await?;

        self.item_label_repo.add_label_to_item(item_id, &label.id).await?;

        Ok(())
    }

    /// Remove label from item
    ///
    /// 从 item_labels 关联表中删除关系
    pub async fn remove_label_from_item(
        &self,
        item_id: &str,
        label_id: &str,
    ) -> Result<(), TodoError> {
        self.item_label_repo.remove_label_from_item(item_id, label_id).await?;

        Ok(())
    }

    /// Get labels by item
    ///
    /// 获取指定 Item 的所有 Labels
    pub async fn get_labels_by_item(
        &self,
        item_id: &str,
    ) -> Result<Vec<crate::entity::LabelModel>, TodoError> {
        let labels = self.item_label_repo.get_labels_by_item(item_id).await?;
        Ok(labels)
    }

    /// Set labels for item
    ///
    /// 批量设置 Item 的 Labels（替换原有 Labels）
    pub async fn set_item_labels(
        &self,
        item_id: &str,
        label_ids: &[String],
    ) -> Result<(), TodoError> {
        self.item_label_repo.set_item_labels(item_id, label_ids).await?;

        Ok(())
    }
}
