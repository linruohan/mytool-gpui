//! Label service for business logic
//!
//! This module provides business logic for Label operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set, prelude::Expr,
};

use crate::{
    entity::{LabelActiveModel, LabelModel, labels, prelude::*},
    error::TodoError,
    repositories::{
        ItemLabelRepository, ItemLabelRepositoryImpl, ItemRepository, ItemRepositoryImpl,
        LabelRepository, LabelRepositoryImpl,
    },
    services::{EventBus, MetricsCollector},
};

/// Service for Label business operations
#[derive(Clone, Debug)]
pub struct LabelService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
    metrics: Arc<MetricsCollector>,
    label_repo: LabelRepositoryImpl,
    item_label_repo: ItemLabelRepositoryImpl,
    item_repo: ItemRepositoryImpl,
}

impl LabelService {
    /// Create a new LabelService
    pub fn new(
        db: Arc<DatabaseConnection>,
        event_bus: Arc<EventBus>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        let label_repo = LabelRepositoryImpl::new(db.clone());
        let item_label_repo = ItemLabelRepositoryImpl::new(db.clone());
        let item_repo = ItemRepositoryImpl::new(db.clone());
        Self { db, event_bus, metrics, label_repo, item_label_repo, item_repo }
    }

    /// Get a label by ID
    pub async fn get_label(&self, id: &str) -> Option<LabelModel> {
        let result: Result<LabelModel, TodoError> = self.label_repo.find_by_id(id).await;
        result.ok()
    }

    /// Insert a new label
    pub async fn insert_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        let _timer = self.metrics.start_timer("insert_label");
        let mut active_label: LabelActiveModel = label.into();
        let label_model = active_label.insert(&*self.db).await?;

        let label_id = label_model.id.clone();
        self.event_bus.publish(crate::services::event_bus::Event::LabelCreated(label_id));

        self.metrics.record_operation("insert_label", 1).await;
        Ok(label_model)
    }

    /// Update an existing label
    pub async fn update_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        let _timer = self.metrics.start_timer("update_label");
        let label_id = label.id.clone();
        let mut active_label: LabelActiveModel = label.into();
        let result = active_label.update(&*self.db).await?;

        self.event_bus.publish(crate::services::event_bus::Event::LabelUpdated(label_id));

        self.metrics.record_operation("update_label", 1).await;
        Ok(result)
    }

    /// Delete a label
    pub async fn delete_label(&self, id: &str) -> Result<u64, TodoError> {
        let _timer = self.metrics.start_timer("delete_label");
        let id_clone = id.to_string();

        let result = self.label_repo.delete(id).await?;
        self.event_bus.publish(crate::services::event_bus::Event::LabelDeleted(id_clone));

        self.metrics.record_operation("delete_label", 1).await;
        Ok(result)
    }

    /// Get or create a label by name
    pub async fn get_or_create_label(
        &self,
        name: &str,
        source_id: &str,
    ) -> Result<LabelModel, TodoError> {
        let _timer = self.metrics.start_timer("get_or_create_label");

        if let Some(label) = self.label_repo.find_by_name(name, source_id).await? {
            return Ok(label);
        }

        let new_label = LabelModel {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            color: "#ff0000".to_string(),
            source_id: Some(source_id.to_string()),
            backend_type: Some("local".to_string()),
            is_deleted: false,
            is_favorite: false,
            item_order: 0,
        };

        let label = self.insert_label(new_label).await?;
        self.metrics.record_operation("get_or_create_label", 1).await;
        Ok(label)
    }

    /// Get labels by source
    pub async fn get_labels_by_source(
        &self,
        source_id: &str,
    ) -> Result<Vec<LabelModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_labels_by_source");
        let labels = self.label_repo.find_by_source(source_id).await?;
        self.metrics.record_operation("get_labels_by_source", labels.len()).await;
        Ok(labels)
    }

    // ==================== Additional Business Logic Methods ====================

    /// Get all labels
    pub async fn get_all_labels(&self) -> Result<Vec<LabelModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_all_labels");
        let labels = self.label_repo.find_all().await?;
        self.metrics.record_operation("get_all_labels", labels.len()).await;
        Ok(labels)
    }

    /// Search labels
    pub async fn search_labels(&self, search_text: &str) -> Result<Vec<LabelModel>, TodoError> {
        let _timer = self.metrics.start_timer("search_labels");
        let search_lower = search_text.to_lowercase();
        let all_labels = self.label_repo.find_all().await?;
        let labels: Vec<LabelModel> = all_labels
            .into_iter()
            .filter(|l| l.name.to_lowercase().contains(&search_lower))
            .collect();
        self.metrics.record_operation("search_labels", labels.len()).await;
        Ok(labels)
    }

    /// Get labels by item
    pub async fn get_labels_by_item(&self, item_id: &str) -> Result<Vec<LabelModel>, TodoError> {
        let _timer = self.metrics.start_timer("get_labels_by_item");
        let labels = self.item_label_repo.get_labels_by_item(item_id).await?;
        self.metrics.record_operation("get_labels_by_item", labels.len()).await;
        Ok(labels)
    }

    /// Add label to item
    pub async fn add_label_to_item(&self, label_id: &str, item_id: &str) -> Result<(), TodoError> {
        let _timer = self.metrics.start_timer("add_label_to_item");
        self.item_label_repo.add_label_to_item(item_id, label_id).await?;
        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id.to_string()));
        self.metrics.record_operation("add_label_to_item", 1).await;
        Ok(())
    }

    /// Remove label from item
    pub async fn remove_label_from_item(
        &self,
        label_id: &str,
        item_id: &str,
    ) -> Result<(), TodoError> {
        let _timer = self.metrics.start_timer("remove_label_from_item");
        self.item_label_repo.remove_label_from_item(item_id, label_id).await?;
        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id.to_string()));
        self.metrics.record_operation("remove_label_from_item", 1).await;
        Ok(())
    }

    /// Get label statistics
    pub async fn get_label_stats(&self, label_id: &str) -> Result<LabelStats, TodoError> {
        let _timer = self.metrics.start_timer("get_label_stats");

        let item_ids = self.item_label_repo.get_items_by_label(label_id).await?;
        let total_items = item_ids.len();

        let mut completed_items = 0;
        let mut pending_items = 0;

        for item_id in &item_ids {
            if let Ok(item) = self.item_repo.find_by_id(item_id).await {
                if item.checked {
                    completed_items += 1;
                } else {
                    pending_items += 1;
                }
            }
        }

        let stats = LabelStats {
            label_id: label_id.to_string(),
            total_items,
            completed_items,
            pending_items,
        };

        self.metrics.record_operation("get_label_stats", 1).await;
        Ok(stats)
    }

    /// Merge labels
    pub async fn merge_labels(
        &self,
        source_label_id: &str,
        target_label_id: &str,
    ) -> Result<(), TodoError> {
        let _timer = self.metrics.start_timer("merge_labels");

        let item_ids = self.item_label_repo.get_items_by_label(source_label_id).await?;
        for item_id in item_ids {
            if !self.item_label_repo.has_label(&item_id, target_label_id).await? {
                self.item_label_repo.add_label_to_item(&item_id, target_label_id).await?;
            }
        }

        self.item_label_repo.remove_all_items_from_label(source_label_id).await?;
        self.delete_label(source_label_id).await?;

        self.metrics.record_operation("merge_labels", 1).await;
        Ok(())
    }
}

/// Label statistics
#[derive(Debug, Clone)]
pub struct LabelStats {
    pub label_id: String,
    pub total_items: usize,
    pub completed_items: usize,
    pub pending_items: usize,
}
