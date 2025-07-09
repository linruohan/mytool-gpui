use crate::entity::prelude::ProjectEntity;
use crate::entity::sources::Column::SourceType;
use crate::entity::{ProjectModel, SourceModel};
use crate::enums::SourceType;
use crate::error::TodoError;
use crate::objects::{BaseTrait, Source};
use crate::{BaseObject, Store};
use sea_orm::{DatabaseConnection, EntityTrait};
use tokio::sync::OnceCell;

#[derive(Clone, Debug)]
pub struct Project {
    pub model: ProjectModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>,
    project_count: Option<usize>,
}

impl Project {
    pub fn id(&self) -> &str {
        &self.model.id
    }
    pub fn set_id(&mut self, id: String) -> &mut Self {
        self.model.id = id;
        self
    }
    pub fn name(&self) -> &str {
        &self.model.name
    }
    pub fn set_name(&mut self, name: String) -> &mut Self {
        self.model.name = name;
        self
    }
    pub fn is_deleted(&self) -> bool {
        self.model.is_deleted
    }
    pub fn set_is_deleted(&mut self, is_deleted: bool) -> &mut Self {
        self.model.is_deleted = is_deleted;
        self
    }
    pub fn is_favorite(&self) -> bool {
        self.model.is_favorite
    }
    pub fn set_is_favorite(&mut self, is_favorite: bool) -> &mut Self {
        self.model.is_favorite = is_favorite;
        self
    }
    pub fn is_archived(&self) -> bool {
        self.model.is_archived
    }
    pub fn set_is_archived(&mut self, is_archived: bool) -> &mut Self {
        self.model.is_archived = is_archived;
        self
    }
    pub fn parent_id(&self) -> Option<&str> {
        self.model.parent_id.as_deref()
    }
    pub fn set_parent_id(&mut self, parent_id: Option<String>) -> &mut Self {
        self.model.parent_id = parent_id;
        self
    }
    pub fn source_id(&self) -> Option<&str> {
        self.model.source_id.as_deref()
    }
    pub fn set_source_id(&mut self, source_id: Option<String>) -> &mut Self {
        self.model.source_id = source_id;
        self
    }
    pub fn display_name(&self) -> String {
        self.model
            .display_name
            .clone()
            .unwrap_or_else(|| self.name().to_string())
    }
    pub fn set_display_name(&mut self, display_name: Option<String>) -> &mut Self {
        self.model.display_name = display_name;
        self
    }
}

impl Project {
    pub fn new(db: DatabaseConnection, model: ProjectModel) -> Self {
        let base = BaseObject::default();
        Self {
            model,
            base,
            db,
            store: OnceCell::new(),
            project_count: None,
        }
    }

    pub async fn store(&self) -> &Store {
        self.store
            .get_or_init(|| async { Store::new(self.db.clone()).await })
            .await
    }
    pub async fn from_db(db: DatabaseConnection, item_id: &str) -> Result<Self, TodoError> {
        let item = ProjectEntity::find_by_id(item_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", item_id)))?;

        Ok(Self::new(db, item))
    }
    pub async fn project_count(&self) -> usize {
        let items = self.store().await.get_items_by_project(self).await;
        items
            .iter()
            .filter(|i| !i.checked() || !i.was_archived())
            .count()
    }
    pub(crate) fn is_inbox_project(&self) -> bool {
        todo!()
    }

    pub async fn source_type(&self) -> SourceType {
        if let Some(source_model) = self.source().await {
            if let Ok(source) = Source::from_db(self.db.clone(), &source_model.id).await {
                return source.source_type();
            }
        }
        SourceType::NONE
    }
    pub(crate) fn update_count(&self) {
        todo!()
    }
    pub async fn parent(&self) -> Option<ProjectModel> {
        let id = self.model.parent_id.as_ref()?;
        self.store().await.get_project(id).await
    }
    pub async fn add_subproject(
        &self,
        subproject: ProjectModel,
    ) -> Result<ProjectModel, TodoError> {
        self.store().await.insert_project(subproject).await
    }
    pub async fn source(&self) -> Option<SourceModel> {
        let id = self.model.source_id.as_ref()?; // Early return if None
        self.store().await.get_source(id).await
    }
}

impl BaseTrait for Project {
    fn id(&self) -> &str {
        &self.model.id
    }

    fn set_id(&mut self, id: &str) {
        self.model.id = id.into();
    }
}
