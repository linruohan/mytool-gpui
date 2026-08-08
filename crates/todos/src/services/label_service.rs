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
};

/// Service for Label business operations
#[derive(Clone, Debug)]
pub struct LabelService {
    db: Arc<DatabaseConnection>,
    label_repo: LabelRepositoryImpl,
}

impl LabelService {
    /// Create a new LabelService
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let label_repo = LabelRepositoryImpl::new(db.clone());
        Self { db, label_repo }
    }

    /// Insert a new label
    pub async fn insert_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        let active_label: LabelActiveModel = label.into();
        let label_model = active_label.insert(&*self.db).await?;
        Ok(label_model)
    }

    /// Update an existing label
    pub async fn update_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
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

        active_label.update(&*self.db).await.map_err(TodoError::from)
    }

    /// Delete a label
    pub async fn delete_label(&self, id: &str) -> Result<u64, TodoError> {
        let deleted = BaseRepository::delete(&self.label_repo, id).await?;
        Ok(if deleted { 1 } else { 0 })
    }

    /// Get or create a label by name
    ///
    /// 🚀 修复：首先全局查找 name（忽略 source_id），避免 UNIQUE constraint 错误
    /// 因为 labels 表有 UNIQUE(name) 约束，相同 name 的 label 只能存在一个
    pub async fn get_or_create_label(
        &self,
        name: &str,
        source_id: &str,
    ) -> Result<LabelModel, TodoError> {
        let existing = LabelEntity::find()
            .filter(labels::Column::Name.eq(name))
            .filter(labels::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?;

        if let Some(label) = existing {
            return Ok(label);
        }

        let new_label = LabelModel {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            source_id: Some(source_id.to_string()),
            ..Default::default()
        };

        self.insert_label(new_label).await
    }

    /// Get all labels
    pub async fn get_all_labels(&self) -> Result<Vec<LabelModel>, TodoError> {
        let labels =
            LabelEntity::find().filter(labels::Column::IsDeleted.eq(false)).all(&*self.db).await?;
        Ok(labels)
    }
}
