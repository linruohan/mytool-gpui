use sea_orm::{DatabaseConnection, EntityTrait};
use tokio::sync::OnceCell;

use crate::{
    BaseObject, Store,
    entity::{ItemModel, ProjectModel, SectionModel, SourceModel, prelude::SectionEntity},
    error::TodoError,
    objects::{BaseTrait, Project},
    utils::Util,
};

#[derive(Clone, Debug)]
pub struct Section {
    pub model: SectionModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>,
    activate_name_editable: bool,
}
impl Section {
    pub fn new(db: DatabaseConnection, model: SectionModel) -> Self {
        let base = BaseObject::default();
        Self { model, base, db, store: OnceCell::new(), activate_name_editable: false }
    }

    pub async fn store(&self) -> &Store {
        self.store.get_or_init(|| async { Store::new(self.db.clone()).await }).await
    }

    pub async fn from_db(db: DatabaseConnection, item_id: &str) -> Result<Self, TodoError> {
        let item = SectionEntity::find_by_id(item_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", item_id)))?;

        Ok(Self::new(db, item))
    }

    pub fn short_name(&self) -> String {
        Util::default().get_short_name(&self.model.name, 0)
    }

    pub async fn project(&self) -> Option<ProjectModel> {
        self.store().await.get_project(self.model.project_id.as_ref()?).await // Assuming Store has a method to get project by ID
    }

    pub async fn items(&self) -> Vec<ItemModel> {
        let mut items = self.store().await.items().await;
        items.sort_by_key(|a| a.child_order);
        items
    }

    pub async fn section_count(&self) -> usize {
        let mut result = 0;
        let items = self.store().await.get_items_by_section(&self.model.id).await;
        result += items.len();
        for item in &items {
            let subitems = self.store().await.get_subitems(&item.id).await;
            result += subitems.len();
        }
        result
    }

    pub async fn add_item_if_not_exist(
        &self,
        item_model: &mut ItemModel,
    ) -> Result<ItemModel, TodoError> {
        match self.get_item(&item_model.id).await {
            Some(item) => Ok(item),
            None => {
                item_model.section_id = Some(self.model.id.clone());
                self.store().await.insert_item(item_model.clone(), true).await
            },
        }
    }

    pub async fn get_item(&self, item_id: &str) -> Option<ItemModel> {
        self.store().await.get_item(item_id).await
    }

    pub async fn get_subitem_size(&self, item_id: &str) -> usize {
        let mut count = 0;
        Box::pin(async move {
            let subitems = self.store().await.get_subitems(item_id).await;
            count += subitems.len();
            for subitem in subitems {
                count += self.get_subitem_size(&subitem.id).await;
            }

            let subitems_uncomplete = self.store().await.get_subitems_uncomplete(item_id).await;
            count += subitems_uncomplete.len();
            for subitem in subitems_uncomplete {
                count += self.get_subitem_size(&subitem.id).await;
            }
        })
        .await;
        count
    }

    pub fn duplicate(&self) -> SectionModel {
        SectionModel {
            name: self.model.name.clone(),
            color: self.model.color.clone(),
            description: self.model.description.clone(),
            ..Default::default()
        }
    }

    pub async fn delete_section(&self) -> Result<(), TodoError> {
        self.store().await.delete_section(self.id()).await
    }

    pub async fn archive_section(&self) -> Result<(), TodoError> {
        self.store().await.archive_section(self.id(), true).await
    }

    pub async fn unarchive_section(&self) -> Result<(), TodoError> {
        self.store().await.archive_section(self.id(), true).await
    }

    pub async fn was_archived(&self) -> bool {
        let Some(project_model) = self.project().await else {
            return self.model.is_archived;
        };
        false
    }

    pub async fn source(&self) -> Option<SourceModel> {
        let project_model = self.project().await?;
        if let Ok(project) = Project::from_db(self.db.clone(), &project_model.id).await {
            return project.source().await;
        }
        None
    }
}

impl BaseTrait for Section {
    fn id(&self) -> &str {
        &self.model.id
    }

    fn set_id(&mut self, id: &str) {
        self.model.id = id.into();
    }
}
