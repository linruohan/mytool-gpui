//! Item service for business logic
//!
//! This module provides business logic for Item operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set, prelude::Expr,
};

use crate::{
    entity::{ItemActiveModel, ItemModel, items, prelude::*},
    error::TodoError,
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

    /// Update an existing item
    pub async fn update_item(
        &self,
        item: ItemModel,
        update_id: &str,
    ) -> Result<ItemModel, TodoError> {
        let item_id = item.id.clone();
        let item_priority = item.priority;
        tracing::info!(
            "ItemService::update_item called for item: {} with priority: {:?}",
            item_id,
            item_priority
        );

        let mut active_model: ItemActiveModel = item.into();
        tracing::info!("Converted to ActiveModel, priority: {:?}", active_model.priority);

        let result = active_model.update(&*self.db).await?;
        tracing::info!(
            "Database update completed for item: {} with priority: {:?}",
            result.id,
            result.priority
        );

        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id));

        Ok(result)
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
            .ok_or_else(|| TodoError::NotFound("item not found".to_string()))?;

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
            .ok_or_else(|| TodoError::NotFound("item not found".to_string()))?;

        ItemEntity::update(ItemActiveModel {
            id: Set(item_id.to_string()),
            project_id: Set(Some(project_id.to_string())),
            section_id: Set(Some(section_id.to_string())),
            ..item.into()
        })
        .exec(&*self.db)
        .await?;

        // 更新子项
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
        // 避免递归调用导致的无限大小 future 问题
        // 改为非递归实现，使用迭代方式处理子项目
        let item_id_clone = item_id.to_string();

        let active_model = ItemActiveModel {
            id: Set(item_id.to_string()),
            checked: Set(checked),
            completed_at: Set(if checked { Some(chrono::Utc::now().naive_utc()) } else { None }),
            ..ItemEntity::find_by_id(item_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| TodoError::NotFound("item not found".to_string()))?
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

        // 不处理父项目的状态更新，避免递归

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
    pub async fn get_all_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_all_items");
        let items = ItemEntity::find().all(&*self.db).await?;
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
        // Sort by due date - 使用类型安全的 due_date() 方法
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
            .ok_or_else(|| TodoError::NotFound("item not found".to_string()))?;

        // 注意：items 表可能没有 is_archived 和 archived_at 字段，这里暂时不更新这些字段
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
            .ok_or_else(|| TodoError::NotFound("item not found".to_string()))?;

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

        // 使用关联表添加关系
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
        // 从关联表中删除关系
        self.item_label_repo.remove_label_from_item(item_id, label_id).await?;

        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id.to_string()));
        Ok(())
    }

    /// Get items by label
    ///
    /// 通过 item_labels 关联表查询具有指定 Label 的所有 Items
    pub async fn get_items_by_label(&self, label_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_items_by_label");

        // 从关联表获取 Item IDs
        let item_ids = self.item_label_repo.get_items_by_label(label_id).await?;

        if item_ids.is_empty() {
            self.metrics.record_operation("get_items_by_label", 0);
            return Ok(vec![]);
        }

        // 查询 Item 详情
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
            .ok_or_else(|| TodoError::NotFound("item not found".to_string()))?;

        // 将 NaiveDateTime 转换为 JsonValue
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
            .filter(items::Column::Checked.eq(false)) // 只返回未完成的任务
            .all(&*self.db)
            .await?
            .into_iter()
            .filter(|item| {
                // 使用类型安全的 due_date() 方法获取日期
                item.due_date()
                    .and_then(|d| d.datetime())
                    .map(|d| d.date() <= today) // 获取due日期小于等于今天的任务（包括过期的和今天到期的）
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
                // 使用类型安全的 due_date() 方法获取日期
                item.due_date().and_then(|d| d.datetime()).map(|d| d < now).unwrap_or(false)
            })
            .collect();
        self.metrics.record_operation("get_overdue_items", items.len()).await;
        Ok(items)
    }
}
