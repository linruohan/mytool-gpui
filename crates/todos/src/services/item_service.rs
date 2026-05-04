//! Item service for business logic
//!
//! This module provides business logic for Item operations,
//! separating it from data access layer.

use std::{sync::Arc, time::Duration};

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QuerySelect, Set, prelude::Expr,
};
use tokio::time;

use crate::{
    entity::{ItemActiveModel, ItemModel, items, prelude::*},
    error::{ErrorContext, TodoError},
    repositories::{
        ItemLabelRepository, ItemLabelRepositoryImpl, ItemRepository, ItemRepositoryImpl,
    },
    services::{EventBus, LabelService, MetricsCollector},
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

    /// 重试机制包装函数
    async fn with_retry<F, T>(&self, operation: F, item_id: String) -> Result<T, TodoError>
    where
        F: Fn()
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, TodoError>> + Send>>,
        T: Send,
    {
        const MAX_RETRIES: usize = 3;
        const RETRY_DELAY: Duration = Duration::from_millis(1000);

        let mut last_error: Option<TodoError> = None;

        for attempt in 0..MAX_RETRIES {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        tracing::info!(
                            "✅ Retry succeeded for item {} on attempt {}",
                            item_id,
                            attempt + 1
                        );
                    }
                    return Ok(result);
                },
                Err(e) => {
                    if e.is_retryable() {
                        if attempt < MAX_RETRIES - 1 {
                            tracing::warn!(
                                "⚠️  Retrying operation for item {} (attempt {} of {}) after \
                                 error: {:?}",
                                item_id,
                                attempt + 1,
                                MAX_RETRIES,
                                e
                            );
                            time::sleep(RETRY_DELAY).await;
                        } else {
                            tracing::error!(
                                "❌ All attempts failed for item {} after {} tries, error: {:?}",
                                item_id,
                                MAX_RETRIES,
                                e
                            );
                        }
                        last_error = Some(e);
                    } else {
                        return Err(e);
                    }
                },
            }
        }

        Err(last_error
            .unwrap_or_else(|| TodoError::DatabaseError("Unknown error during retry".to_string())))
    }

    /// Update an existing item
    pub async fn update_item(
        &self,
        item: ItemModel,
        update_id: &str,
    ) -> Result<ItemModel, TodoError> {
        let item_id = item.id.clone();
        let item_priority = item.priority;
        let item_content = item.content.clone();
        let item_due = item.due.clone();
        tracing::info!(
            "ItemService::update_item called for item: {} with priority: {:?}, content: '{}', \
             due: {:?}",
            item_id,
            item_priority,
            item_content,
            item_due
        );

        let now = chrono::Utc::now().naive_utc();

        let update_result = self
            .with_retry(
                || {
                    let db = self.db.clone();
                    let item_clone = item.clone();
                    let item_id_clone = item_id.clone();
                    let now_clone = now;

                    Box::pin(async move {
                        tracing::info!("🔍 Executing update_many for item: {}", item_id_clone);

                        let existing_item = items::Entity::find()
                            .filter(items::Column::Id.eq(item_id_clone.clone()))
                            .one(&*db)
                            .await?;

                        if existing_item.is_none() {
                            tracing::error!(
                                "❌ Item {} not found in database! Cannot update.",
                                item_id_clone
                            );
                            return Err(
                                TodoError::not_found("Item").with_entity("Item", &item_id_clone)
                            );
                        }

                        tracing::info!(
                            "✅ Item {} found in database, proceeding with update",
                            item_id_clone
                        );

                        let result = items::Entity::update_many()
                            .col_expr(
                                items::Column::Content,
                                Expr::value(item_clone.content.clone()),
                            )
                            .col_expr(
                                items::Column::Description,
                                Expr::value(item_clone.description.clone()),
                            )
                            .col_expr(items::Column::Due, Expr::value(item_clone.due.clone()))
                            .col_expr(items::Column::UpdatedAt, Expr::value(now_clone))
                            .col_expr(
                                items::Column::SectionId,
                                Expr::value(item_clone.section_id.clone()),
                            )
                            .col_expr(
                                items::Column::ProjectId,
                                Expr::value(item_clone.project_id.clone()),
                            )
                            .col_expr(
                                items::Column::ParentId,
                                Expr::value(item_clone.parent_id.clone()),
                            )
                            .col_expr(items::Column::Priority, Expr::value(item_clone.priority))
                            .col_expr(
                                items::Column::ChildOrder,
                                Expr::value(item_clone.child_order),
                            )
                            .col_expr(items::Column::DayOrder, Expr::value(item_clone.day_order))
                            .col_expr(items::Column::Checked, Expr::value(item_clone.checked))
                            .col_expr(items::Column::IsDeleted, Expr::value(item_clone.is_deleted))
                            .col_expr(items::Column::Collapsed, Expr::value(item_clone.collapsed))
                            .col_expr(items::Column::Pinned, Expr::value(item_clone.pinned))
                            .col_expr(items::Column::Labels, Expr::value(item_clone.labels.clone()))
                            .col_expr(
                                items::Column::ExtraData,
                                Expr::value(item_clone.extra_data.clone()),
                            )
                            .col_expr(
                                items::Column::ItemType,
                                Expr::value(item_clone.item_type.clone()),
                            )
                            .filter(items::Column::Id.eq(item_id_clone.clone()))
                            .exec(&*db)
                            .await;

                        match &result {
                            Ok(res) => {
                                tracing::info!(
                                    "✅ update_many success, rows affected: {}",
                                    res.rows_affected
                                );
                            },
                            Err(e) => {
                                tracing::error!(
                                    "❌ update_many failed for item {}: {:?}",
                                    item_id_clone,
                                    e
                                );
                            },
                        }

                        result.map_err(TodoError::from)
                    })
                },
                item_id.clone(),
            )
            .await?;

        let updated_item = self
            .with_retry(
                || {
                    let item_id_clone = item_id.clone();
                    let service = self.clone();

                    Box::pin(async move {
                        tracing::info!("🔍 Fetching updated item from database: {}", item_id_clone);
                        service.get_item(&item_id_clone).await.ok_or_else(|| {
                            TodoError::not_found("Updated item").with_entity("Item", &item_id_clone)
                        })
                    })
                },
                item_id.clone(),
            )
            .await?;

        tracing::info!(
            "✅ Database Update Success - Item ID: {}, Content: '{}', Priority: {:?}, Due: {:?}",
            updated_item.id,
            updated_item.content,
            updated_item.priority,
            updated_item.due
        );

        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id));

        Ok(updated_item)
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

                crate::entity::items::Entity::update_many()
                    .col_expr(items::Column::Checked, Expr::value(checked_value))
                    .col_expr(items::Column::CompletedAt, Expr::value(completed_at_value))
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
    /// 关键修复：同时加载每个 item 的 labels 并填充到 ItemModel.labels 字段
    /// 这样 UI 可以直接从 item.labels 获取标签列表，而不需要额外查询
    pub async fn get_all_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_all_items");
        let mut items = ItemEntity::find().all(&*self.db).await?;

        tracing::info!("get_all_items: loaded {} items from database", items.len());

        for item in &mut items {
            tracing::debug!("get_all_items: item {} has due: {:?}", item.id, item.due);

            match self.get_labels_by_item(&item.id).await {
                Ok(labels) => {
                    let label_ids: Vec<String> = labels.iter().map(|l| l.id.clone()).collect();
                    item.labels = Some(label_ids.join(";"));
                },
                Err(e) => {
                    tracing::warn!("Failed to load labels for item {}: {:?}", item.id, e);
                    item.labels = None;
                },
            }
        }

        self.metrics.record_operation("get_all_items", items.len());
        Ok(items)
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
    pub async fn archive_item(&self, item_id: &str, archived: bool) -> Result<(), TodoError> {
        let item = self
            .get_item(item_id)
            .await
            .ok_or_else(|| TodoError::not_found("Item").with_entity("Item", item_id))?;

        ItemEntity::update(ItemActiveModel { id: Set(item_id.to_string()), ..item.into() })
            .exec(&*self.db)
            .await?;

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
