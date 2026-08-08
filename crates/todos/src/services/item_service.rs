//! Item service for business logic
//!
//! This module provides business logic for Item operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, prelude::Expr,
};

use crate::{
    entity::{ItemActiveModel, ItemModel, items, prelude::*},
    error::TodoError,
    repositories::{
        BaseRepository, ItemLabelRepository, ItemLabelRepositoryImpl, ItemQueryRepository,
        ItemRepositoryImpl,
    },
    services::{EventBus, LabelService},
    utils::retry_with_context,
};

/// Service for Item business operations
#[derive(Clone, Debug)]
pub struct ItemService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
    label_service: Arc<LabelService>,
    item_repo: ItemRepositoryImpl,
    item_label_repo: ItemLabelRepositoryImpl,
}

impl ItemService {
    /// Create a new ItemService
    pub fn new(
        db: Arc<DatabaseConnection>,
        event_bus: Arc<EventBus>,
        label_service: Arc<LabelService>,
    ) -> Self {
        let item_repo = ItemRepositoryImpl::new(db.clone());
        let item_label_repo = ItemLabelRepositoryImpl::new(db.clone());
        Self { db, event_bus, label_service, item_repo, item_label_repo }
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

        let item_id = item_model.id.clone();
        self.publish_item_position(&item_model);
        self.event_bus.publish(crate::services::event_bus::Event::ItemCreated(item_id));

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

        self.execute_item_update(&item, now).await?;

        let updated_item = self.fetch_updated_item(&item_id).await?;

        tracing::info!(
            "✅ 更新成功 - id: {}, content: '{}', priority: {:?}",
            updated_item.id,
            updated_item.content,
            updated_item.priority
        );

        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id));

        Ok(updated_item)
    }

    /// 执行数据库更新操作
    async fn execute_item_update(
        &self,
        item: &ItemModel,
        now: chrono::NaiveDateTime,
    ) -> Result<(), TodoError> {
        let item_id = item.id.clone();
        let db = self.db.clone();
        let item_clone = item.clone();

        retry_with_context("execute_item_update", "Item", &item_id, || {
            let db = db.clone();
            let item = item_clone.clone();
            let item_id = item_id.clone();

            Box::pin(async move {
                self.verify_item_exists(&db, &item_id).await?;
                self.update_item_fields(&db, &item, now).await
            })
        })
        .await
    }

    /// 验证任务是否存在
    async fn verify_item_exists(
        &self,
        db: &DatabaseConnection,
        item_id: &str,
    ) -> Result<(), TodoError> {
        let exists =
            items::Entity::find().filter(items::Column::Id.eq(item_id)).one(db).await?.is_some();

        if !exists {
            return Err(TodoError::not_found("Item").with_entity("Item", item_id));
        }

        Ok(())
    }

    /// 更新任务字段到数据库
    async fn update_item_fields(
        &self,
        db: &DatabaseConnection,
        item: &ItemModel,
        now: chrono::NaiveDateTime,
    ) -> Result<(), TodoError> {
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
            .exec(db)
            .await;

        match &result {
            Ok(res) => {
                tracing::info!("✅ update_many 成功, 影响行数: {}", res.rows_affected);
            },
            Err(e) => {
                tracing::error!("❌ update_many 失败 for item {}: {:?}", item.id, e);
            },
        }

        result.map(|_| ()).map_err(TodoError::from)
    }

    /// 获取更新后的任务
    async fn fetch_updated_item(&self, item_id: &str) -> Result<ItemModel, TodoError> {
        let service = self.clone();
        let item_id = item_id.to_string();

        retry_with_context("fetch_updated_item", "Item", &item_id, || {
            let service = service.clone();
            let item_id = item_id.clone();

            Box::pin(async move {
                service.get_item(&item_id).await.ok_or_else(|| {
                    TodoError::not_found("Updated item").with_entity("Item", &item_id)
                })
            })
        })
        .await
    }

    /// Delete an item and its children
    ///
    /// 同时删除 item_labels 关联表中的记录（通过数据库级联删除）
    pub async fn delete_item(&self, item_id: &str) -> Result<(), TodoError> {
        let item_id_clone = item_id.to_string();

        let mut items_to_delete = vec![item_id.to_string()];

        while let Some(current_id) = items_to_delete.pop() {
            let subitems =
                ItemQueryRepository::find_by_parent(&self.item_repo, &current_id).await?;

            for item in subitems {
                items_to_delete.push(item.id);
            }

            BaseRepository::delete(&self.item_repo, &current_id).await?;
        }

        self.event_bus.publish(crate::services::event_bus::Event::ItemDeleted(item_id_clone));

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

        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id.to_string()));

        Ok(())
    }

    /// Complete/uncomplete an item
    pub async fn complete_item(
        &self,
        item_id: &str,
        checked: bool,
        complete_subitems: bool,
    ) -> Result<(), TodoError> {
        let item_id_clone = item_id.to_string();

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

        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id_clone));

        Ok(())
    }

    // ==================== Helper Methods ====================

    fn publish_item_position(&self, item: &ItemModel) {
        if let Some(project_id) = &item.project_id
            && let Some(section_id) = &item.section_id
        {
            self.publish_item_position_update(project_id, section_id);
        }
    }

    fn publish_item_position_update(&self, project_id: &str, section_id: &str) {
        self.event_bus.publish(crate::services::event_bus::Event::ItemsPositionUpdated(
            project_id.to_string(),
            section_id.to_string(),
        ));
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

        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id.to_string()));
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

        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id.to_string()));
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

        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id.to_string()));
        Ok(())
    }
}
