//! Optimized query service for batch operations and concurrent processing

use std::sync::Arc;

use futures::future::try_join_all;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::{
    entity::{
        ItemModel, LabelModel, ProjectModel, SectionModel, items, labels, projects, sections,
    },
    error::TodoError,
};

pub struct QueryService {
    db: Arc<DatabaseConnection>,
}

impl Clone for QueryService {
    fn clone(&self) -> Self {
        Self { db: self.db.clone() }
    }
}

impl std::fmt::Debug for QueryService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryService").finish()
    }
}

impl QueryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn batch_load_items(&self, ids: Vec<String>) -> Result<Vec<ItemModel>, TodoError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let items = items::Entity::find()
            .filter(items::Column::Id.is_in(ids))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?;

        Ok(items)
    }

    pub async fn batch_load_projects(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<ProjectModel>, TodoError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let projects = projects::Entity::find()
            .filter(projects::Column::Id.is_in(ids))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?;

        Ok(projects)
    }

    pub async fn batch_load_sections(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<SectionModel>, TodoError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let sections = sections::Entity::find()
            .filter(sections::Column::Id.is_in(ids))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?;

        Ok(sections)
    }

    pub async fn batch_load_labels(&self, ids: Vec<String>) -> Result<Vec<LabelModel>, TodoError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }

        let labels = labels::Entity::find()
            .filter(labels::Column::Id.is_in(ids))
            .all(&*self.db)
            .await
            .map_err(|e| TodoError::DatabaseError(e.to_string()))?;

        Ok(labels)
    }
}
