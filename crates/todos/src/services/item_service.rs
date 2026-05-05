//! Item service for business logic
//!
//! This module provides business logic for Item operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QuerySelect, Set, prelude::Expr,
};

use crate::{
    entity::{ItemActiveModel, ItemModel, items, prelude::*},
    error::{ErrorContext, TodoError},
    repositories::{
        ItemLabelRepository, ItemLabelRepositoryImpl, ItemRepository, ItemRepositoryImpl,
    },
    services::{EventBus, LabelService, MetricsCollector},
    utils::{retry_operation, retry_with_context},
};

/// Service for Item business operations
#[derive(Clone, Debug)]
pub struct ItemService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
    metrics: Arc<MetricsCollector>,
    label_service: Arc<LabelService>,
    item_repo: ItemRepositoryImpl,
    item_label_repo: ItemLabelRepositoryImpl,
}

impl ItemService {
    /// Create a new ItemService
    pub fn new(
        db: Arc<DatabaseConnection>,
        event_bus: Arc<EventBus>,
        metrics: Arc<MetricsCollector>,
        label_service: Arc<LabelService>,
    ) -> Self {
        let item_repo = ItemRepositoryImpl::new(db.clone());
        let item_label_repo = ItemLabelRepositoryImpl::new(db.clone());
        Self { db, event_bus, metrics, label_service, item_repo, item_label_repo }
    }

    /// Get an item by ID
    pub async fn get_item(&self, id: &str) -> Option<ItemModel> {
        let result: Result<ItemModel, TodoError> = self.item_repo.find_by_id(id).await;
        result.ok()
    }

    /// Insert a new item
    pub async fn insert_item(&self, item: ItemModel, insert: bool) -> Result<ItemModel, TodoError> {
        let mut active_model: ItemActiveModel = item.into();
        let item_model = active_model.insert(&*self.db).await?;

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
        update_id: &str,
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
            let subitems = self.item_repo.find_by_parent(&current_id).await?;

            for item in subitems {
                items_to_delete.push(item.id);
            }

            self.item_repo.delete(&current_id).await?;
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

    /// Move item to another project/section
    pub async fn move_item(
        &self,
        item_id: &str,
        project_id: &str,
        section_id: &str,
    ) -> Result<(), TodoError> {
        let item = self
            .get_item(item_id)
            .await
            .ok_or_else(|| TodoError::not_found("Item").with_entity("Item", item_id))?;

        ItemEntity::update(ItemActiveModel {
            id: Set(item_id.to_string()),
            project_id: Set(Some(project_id.to_string())),
            section_id: Set(Some(section_id.to_string())),
            ..item.into()
        })
        .exec(&*self.db)
        .await?;

        ItemEntity::update_many()
            .col_expr(items::Column::ProjectId, Expr::value(project_id.to_string()))
            .col_expr(items::Column::SectionId, Expr::value(section_id.to_string()))
            .col_expr(items::Column::UpdatedAt, Expr::value(chrono::Utc::now().naive_utc()))
            .filter(items::Column::ParentId.eq(item_id.to_string()))
            .exec(&*self.db)
            .await?;

        self.publish_item_position_update(project_id, section_id);
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
            let subitems = self.item_repo.find_by_parent(item_id).await?;
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
        let _timer = self.metrics.start_timer("get_items_by_project");
        let items = ItemEntity::find()
            .filter(items::Column::ProjectId.eq(project_id))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("get_items_by_project", items.len());
        Ok(items)
    }

    /// Get all items in a section
    pub async fn get_items_by_section(
        &self,
        section_id: &str,
    ) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_items_by_section");
        let items = ItemEntity::find()
            .filter(items::Column::SectionId.eq(section_id))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("get_items_by_section", items.len());
        Ok(items)
    }

    /// Get all subitems of an item
    pub async fn get_subitems(&self, item_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_subitems");
        let items =
            ItemEntity::find().filter(items::Column::ParentId.eq(item_id)).all(&*self.db).await?;
        self.metrics.record_operation("get_subitems", items.len());
        Ok(items)
    }

    /// Get all pinned items
    pub async fn get_pinned_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_pinned_items");
        let items =
            ItemEntity::find().filter(items::Column::Pinned.eq(true)).all(&*self.db).await?;
        self.metrics.record_operation("get_pinned_items", items.len());
        Ok(items)
    }

    /// Get all incomplete pinned items
    pub async fn get_incomplete_pinned_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_incomplete_pinned_items");
        let items = ItemEntity::find()
            .filter(items::Column::Pinned.eq(true))
            .filter(items::Column::Checked.eq(false))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("get_incomplete_pinned_items", items.len());
        Ok(items)
    }

    /// Get all completed items
    pub async fn get_completed_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_completed_items");
        let items =
            ItemEntity::find().filter(items::Column::Checked.eq(true)).all(&*self.db).await?;
        self.metrics.record_operation("get_completed_items", items.len());
        Ok(items)
    }

    /// Get all incomplete items
    pub async fn get_incomplete_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_incomplete_items");
        let items =
            ItemEntity::find().filter(items::Column::Checked.eq(false)).all(&*self.db).await?;
        self.metrics.record_operation("get_incomplete_items", items.len());
        Ok(items)
    }

    /// Get all items (including completed and incomplete)
    ///
    /// 🚀 关键修复：使用批量加载 labels，避免 N+1 查询问题
    /// 原来：每个 item 都会触发一次 get_labels_by_item 查询
    /// 现在：只触发一次 get_all_item_labels 查询，然后将结果填充到 items
    pub async fn get_all_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_all_items");
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

        self.metrics.record_operation("get_all_items", result.len());
        Ok(result)
    }

    /// Get all scheduled items (items with due date that are not completed)
    /// 使用类型安全的 due_date() 方法替代手动 JSON 解析
    pub async fn get_scheduled_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_scheduled_items");
        let mut items: Vec<ItemModel> = ItemEntity::find()
            .filter(items::Column::Due.is_not_null())
            .filter(items::Column::Checked.eq(false))
            .all(&*self.db)
            .await?;
        items.sort_by(|a, b| {
            let a_date = a.due_date().and_then(|d| d.datetime());
            let b_date = b.due_date().and_then(|d| d.datetime());
            a_date.cmp(&b_date)
        });
        self.metrics.record_operation("get_scheduled_items", items.len());
        Ok(items)
    }

    /// Get items by search text
    pub async fn search_items(&self, search_text: &str) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("search_items");
        let search_lower = search_text.to_lowercase();
        let items = ItemEntity::find()
            .filter(items::Column::Content.contains(&search_lower))
            .all(&*self.db)
            .await?;
        self.metrics.record_operation("search_items", items.len());
        Ok(items)
    }

    /// Archive an item
    ///
    /// 注意：Items 表目前没有 is_archived 字段，此方法暂时是空操作
    /// 归档功能对 Items 的语义由 is_deleted 字段处理
    pub async fn archive_item(&self, item_id: &str, archived: bool) -> Result<(), TodoError> {
        tracing::warn!(
            "archive_item called for item {} with archived={}, but Items table has no is_archived \
             field. This is a no-op for now. Use is_deleted to soft-delete items instead.",
            item_id,
            archived
        );

        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id.to_string()));
        Ok(())
    }

    /// Duplicate an item
    pub async fn duplicate_item(&self, item_id: &str) -> Result<ItemModel, TodoError> {
        let item = self
            .get_item(item_id)
            .await
            .ok_or_else(|| TodoError::not_found("Item").with_entity("Item", item_id))?;

        let mut new_item = item.clone();
        new_item.id = uuid::Uuid::new_v4().to_string();
        new_item.content = format!("{} (copy)", item.content);
        new_item.added_at = chrono::Utc::now().naive_utc();
        new_item.completed_at = None;
        new_item.checked = false;

        self.insert_item(new_item, true).await
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

    /// Get items by label
    ///
    /// 通过 item_labels 关联表查询具有指定 Label 的所有 Items
    pub async fn get_items_by_label(&self, label_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_items_by_label");

        let item_ids = self.item_label_repo.get_items_by_label(label_id).await?;

        if item_ids.is_empty() {
            self.metrics.record_operation("get_items_by_label", 0);
            return Ok(vec![]);
        }

        let items =
            ItemEntity::find().filter(items::Column::Id.is_in(item_ids)).all(&*self.db).await?;

        self.metrics.record_operation("get_items_by_label", items.len());
        Ok(items)
    }

    /// Get labels by item
    ///
    /// 获取指定 Item 的所有 Labels
    pub async fn get_labels_by_item(
        &self,
        item_id: &str,
    ) -> Result<Vec<crate::entity::LabelModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_labels_by_item");

        let labels = self.item_label_repo.get_labels_by_item(item_id).await?;

        self.metrics.record_operation("get_labels_by_item", labels.len());
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

    /// Check if item has label
    ///
    /// 检查 Item 是否有指定的 Label
    pub async fn item_has_label(&self, item_id: &str, label_id: &str) -> Result<bool, TodoError> {
        self.item_label_repo.has_label(item_id, label_id).await
    }

    /// Set due date for item
    pub async fn set_due_date(
        &self,
        item_id: &str,
        due_date: Option<chrono::NaiveDateTime>,
    ) -> Result<(), TodoError> {
        let item = self
            .get_item(item_id)
            .await
            .ok_or_else(|| TodoError::not_found("Item").with_entity("Item", item_id))?;

        let due_json = due_date.map(|d| serde_json::Value::String(d.to_string()));

        ItemEntity::update(ItemActiveModel {
            id: Set(item_id.to_string()),
            due: Set(due_json),
            ..item.into()
        })
        .exec(&*self.db)
        .await?;

        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id.to_string()));
        Ok(())
    }

    /// Get items due today and overdue items
    /// 使用类型安全的 due_date() 方法替代手动 JSON 解析
    pub async fn get_items_due_today(&self) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_items_due_today");
        let today = chrono::Utc::now().naive_utc().date();
        let items: Vec<ItemModel> = ItemEntity::find()
            .filter(items::Column::Due.is_not_null())
            .filter(items::Column::Checked.eq(false))
            .all(&*self.db)
            .await?
            .into_iter()
            .filter(|item| {
                item.due_date()
                    .and_then(|d| d.datetime())
                    .map(|d| d.date() <= today)
                    .unwrap_or(false)
            })
            .collect();
        self.metrics.record_operation("get_items_due_today", items.len()).await;
        Ok(items)
    }

    /// Get overdue items
    /// 使用类型安全的 due_date() 方法替代手动 JSON 解析
    pub async fn get_overdue_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_overdue_items");
        let now = chrono::Utc::now().naive_utc();
        let items: Vec<ItemModel> = ItemEntity::find()
            .filter(items::Column::Due.is_not_null())
            .filter(items::Column::Checked.eq(false))
            .all(&*self.db)
            .await?
            .into_iter()
            .filter(|item| {
                item.due_date().and_then(|d| d.datetime()).map(|d| d < now).unwrap_or(false)
            })
            .collect();
        self.metrics.record_operation("get_overdue_items", items.len()).await;
        Ok(items)
    }
}
