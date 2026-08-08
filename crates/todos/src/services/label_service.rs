//! Label service for business logic
//!
//! This module provides business logic for Label operations,
//! separating it from data access layer.

use std::sync::Arc;

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::{
    entity::{LabelActiveModel, LabelModel, labels, prelude::*},
    error::TodoError,
    repositories::{BaseRepository, LabelRepositoryImpl},
    services::EventBus,
};

/// Service for Label business operations
#[derive(Clone, Debug)]
pub struct LabelService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
    label_repo: LabelRepositoryImpl,
}

impl LabelService {
    /// Create a new LabelService
    pub fn new(db: Arc<DatabaseConnection>, event_bus: Arc<EventBus>) -> Self {
        let label_repo = LabelRepositoryImpl::new(db.clone());
        Self { db, event_bus, label_repo }
    }

    /// Insert a new label
    pub async fn insert_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        let active_label: LabelActiveModel = label.into();
        let label_model = active_label.insert(&*self.db).await?;

        let label_id = label_model.id.clone();
        self.event_bus.publish(crate::services::event_bus::Event::LabelCreated(label_id));

        Ok(label_model)
    }

    /// Update an existing label
    pub async fn update_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        let label_id = label.id.clone();

        // 显式设置需要更新的字段
        let active_label = LabelActiveModel {
            id: Set(label.id),
            name: Set(label.name),
            color: Set(label.color),
            item_order: Set(label.item_order),
            is_deleted: Set(label.is_deleted),
            is_favorite: Set(label.is_favorite),
            backend_type: Set(label.backend_type),
            source_id: Set(label.source_id),
        };

        let result = active_label.update(&*self.db).await?;

        self.event_bus.publish(crate::services::event_bus::Event::LabelUpdated(label_id));

        Ok(result)
    }

    /// Delete a label
    pub async fn delete_label(&self, id: &str) -> Result<u64, TodoError> {
        let id_clone = id.to_string();

        let deleted = BaseRepository::delete(&self.label_repo, id).await?;
        self.event_bus.publish(crate::services::event_bus::Event::LabelDeleted(id_clone));

        Ok(if deleted { 1 } else { 0 })
    }

    /// Get or create a label by name
    ///
    /// 🚀 修复：首先全局查找 name（忽略 source_id），避免 UNIQUE constraint 错误
    /// 因为 labels 表有 UNIQUE(name) 约束，相同 name 的 label 只能存在一个
    pub async fn get_or_create_label(
        &self,
        name: &str,
        _source_id: &str,
    ) -> Result<LabelModel, TodoError> {
        // 🚀 修复：首先只按 name 查找（全局查找），不指定 source_id
        // 这样可以找到已存在的同名 label，避免尝试插入导致 UNIQUE constraint 错误
        if let Some(label) = self.find_label_by_name_global(name).await? {
            return Ok(label);
        }

        let new_label = LabelModel {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            color: "#ff0000".to_string(),
            source_id: Some(_source_id.to_string()),
            backend_type: Some("local".to_string()),
            is_deleted: false,
            is_favorite: false,
            item_order: 0,
        };

        let label = self.insert_label(new_label).await?;
        Ok(label)
    }

    /// 全局查找 label by name（不考虑 source_id）
    async fn find_label_by_name_global(&self, name: &str) -> Result<Option<LabelModel>, TodoError> {
        LabelEntity::find()
            .filter(labels::Column::Name.eq(name))
            .one(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))
    }

    /// Get all labels
    pub async fn get_all_labels(&self) -> Result<Vec<LabelModel>, TodoError> {
        let labels = BaseRepository::find_all(&self.label_repo).await?;
        Ok(labels)
    }
}
