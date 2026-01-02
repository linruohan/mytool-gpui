use std::{error::Error, fmt};

use futures::stream::{self, StreamExt};
use sea_orm::{DatabaseConnection, EntityTrait};
use tokio::sync::OnceCell;

use crate::{
    BaseObject, Store,
    entity::{ItemModel, ProjectModel, SectionModel, SourceModel, prelude::ProjectEntity},
    enums::{ProjectIconStyle, ProjectViewStyle, SourceType},
    error::TodoError,
    objects::{BaseTrait, Item, Source},
    utils::Util,
};

#[derive(Clone, Debug)]
pub struct Project {
    pub model: ProjectModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>,
    project_count: Option<usize>,
}
impl Project {
    pub fn icon_style(&self) -> ProjectIconStyle {
        self.model.icon_style.as_deref().map_or(ProjectIconStyle::PROGRESS, ProjectIconStyle::parse)
    }

    pub fn set_icon_style(&mut self, icon_style: ProjectIconStyle) -> &mut Self {
        self.model.icon_style = Some(icon_style.to_string());
        self
    }

    pub fn backend_type(&self) -> SourceType {
        self.model.backend_type.as_deref().map_or(SourceType::NONE, SourceType::parse)
    }

    pub fn set_backend_type(&mut self, backend_type: SourceType) -> &mut Self {
        self.model.backend_type = Some(backend_type.to_string());
        self
    }

    pub fn view_style(&self) -> ProjectViewStyle {
        self.model.view_style.as_deref().map_or(ProjectViewStyle::LIST, ProjectViewStyle::parse)
    }

    pub fn set_view_style(&mut self, view_style: ProjectViewStyle) -> &mut Self {
        self.model.view_style = Some(view_style.to_string());
        self
    }
}

impl Project {
    pub fn new(db: DatabaseConnection, model: ProjectModel) -> Self {
        let base = BaseObject::default();
        Self { model, base, db, store: OnceCell::new(), project_count: None }
    }

    pub async fn store(&self) -> &Store {
        self.store.get_or_init(|| async { Store::new(self.db.clone()).await }).await
    }

    pub async fn from_db(db: DatabaseConnection, project_id: &str) -> Result<Self, TodoError> {
        let item = ProjectEntity::find_by_id(project_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", project_id)))?;

        Ok(Self::new(db, item))
    }

    pub async fn source_type(&self) -> SourceType {
        if let Some(source_model) = self.source().await
            && let Ok(source) = Source::from_db(self.db.clone(), &source_model.id).await
        {
            return source.source_type();
        }
        SourceType::NONE
    }

    pub async fn source(&self) -> Option<SourceModel> {
        self.store().await.get_source(self.model.source_id.as_ref()?).await
    }

    pub fn color_hex(&self) -> String {
        self.model
            .color
            .as_ref()
            .map(|color| Util::default().get_color(color.to_string()))
            .unwrap_or_default()
    }

    pub fn view_id(&self) -> String {
        format!("project-{}", self.model.id)
    }

    pub fn parent_id_string(&self) -> String {
        self.model.parent_id.as_ref().map_or_else(String::new, |id| id.to_string())
    }

    pub fn short_name(&self) -> String {
        Util::default().get_short_name(&self.model.name, 0)
    }

    pub fn is_inbox_project(&self) -> bool {
        // return id == Services.Settings.get_default ().settings.get_string
        // ("local-inbox-project-id");
        self.model.id == "inbox"
    }

    pub async fn sections(&self) -> Vec<SectionModel> {
        self.store().await.get_sections_by_project(&self.model.id).await
    }

    pub async fn items(&self) -> Vec<ItemModel> {
        let mut items = self.store().await.get_items_by_project(&self.model.id).await;
        items.sort_by(|a, b| a.child_order.cmp(&b.child_order));
        items
    }

    pub async fn sections_archived(&self) -> Vec<SectionModel> {
        self.store().await.get_sections_archived_by_project(&self.model.id).await
    }

    pub async fn items_checked(&self) -> Vec<ItemModel> {
        self.store().await.get_items_checked_by_project(&self.model.id).await
    }

    pub async fn all_items(&self) -> Vec<ItemModel> {
        self.store().await.get_items_by_project(&self.model.id).await
    }

    pub async fn items_pinned(&self) -> Vec<ItemModel> {
        self.store().await.get_items_by_project_pinned(&self.model.id).await
    }

    pub async fn subprojects(&self) -> Vec<ProjectModel> {
        self.store().await.get_subprojects(&self.model.id).await
    }

    pub async fn parent(&self) -> Option<ProjectModel> {
        self.store().await.get_project(self.model.parent_id.as_ref()?).await
    }

    pub fn is_deck(&self) -> bool {
        self.model.id.contains("deck--board")
    }

    pub async fn project_count(&self) -> usize {
        let items_model = self.items().await;
        stream::iter(items_model)
            .filter_map(async move |model| {
                if let Ok(item) = Item::from_db(self.db.clone(), &model.id).await
                    && !item.model.checked
                    && !item.was_archived().await
                {
                    return Some(model);
                }
                None
            })
            .count()
            .await
    }

    pub async fn project_percentage(&self) -> f32 {
        let count = self.project_count().await;
        let total = self.items().await.len();
        count as f32 / total as f32
    }

    pub async fn update_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        self.store().await.update_project(project).await
    }

    pub async fn update(
        &self,
        use_timeout: bool,
        show_loading: bool,
    ) -> Result<ProjectModel, TodoError> {
        self.store().await.update_project(self.model.clone()).await
    }

    pub async fn get_subproject(&self, subproject_id: &str) -> Option<ProjectModel> {
        let subprojects = self.subprojects().await;
        subprojects.iter().find(|p| p.id == subproject_id).cloned()
    }

    pub async fn add_subproject_if_not_exists(
        &self,
        pro: &mut ProjectModel,
    ) -> Result<ProjectModel, TodoError> {
        match self.get_subproject(&pro.id).await {
            Some(subproject) => Ok(subproject),
            None => {
                pro.parent_id = Some(self.model.id.clone());
                self.store().await.insert_project(pro.clone()).await
            },
        }
    }

    pub fn set_parent(&mut self, parent: ProjectModel) {
        self.model.parent_id = Some(parent.id.clone());
    }

    pub async fn add_subproject(
        &self,
        subproject: ProjectModel,
    ) -> Result<ProjectModel, TodoError> {
        self.store().await.insert_project(subproject).await
    }

    pub async fn get_section(&self, section_id: &str) -> Option<SectionModel> {
        self.store().await.get_section(section_id).await
    }

    pub async fn add_section_if_not_exists(
        &self,
        section_model: &mut SectionModel,
    ) -> Result<SectionModel, TodoError> {
        match self.get_section(&section_model.id).await {
            Some(section) => Ok(section),
            None => {
                section_model.project_id = Some(section_model.id.clone());
                let section_order = self.sections().await.len() + 1;
                section_model.section_order = Some(section_order as i32);
                self.store().await.insert_section(section_model.clone()).await
            },
        }
    }

    pub async fn add_section(
        &self,
        section_model: SectionModel,
    ) -> Result<SectionModel, TodoError> {
        self.store().await.insert_section(section_model).await
    }

    pub async fn get_item(&self, item_id: &str) -> Option<ItemModel> {
        self.store().await.get_item(item_id).await
    }

    pub async fn add_item_if_not_exists(
        &self,
        item_model: &mut ItemModel,
    ) -> Result<ItemModel, TodoError> {
        match self.get_item(&item_model.id).await {
            Some(item) => Ok(item),
            None => {
                item_model.project_id = Some(item_model.id.clone());
                self.store().await.insert_item(item_model.clone(), true).await
            },
        }
    }

    pub async fn add_item(&self, item: ItemModel) -> Result<ItemModel, TodoError> {
        self.store().await.insert_item(item.clone(), true).await
    }

    pub async fn delete_project(&self) -> Result<(), TodoError> {
        self.store().await.delete_project(&self.model.id).await
    }

    pub async fn archive_project(&self) -> Result<(), TodoError> {
        self.store().await.archive_project(&self.model.id).await
    }

    pub async fn unarchive_project(&self) -> Result<(), TodoError> {
        self.store().await.archive_project(&self.model.id).await
    }

    pub fn duplicate(&self) -> ProjectModel {
        ProjectModel {
            name: self.model.name.clone(),
            due_date: self.model.due_date.clone(),
            color: self.model.color.clone(),
            emoji: self.model.emoji.clone(),
            description: self.model.description.clone(),
            icon_style: self.model.icon_style.clone(),
            backend_type: self.model.backend_type.clone(),
            source_id: self.model.source_id.clone(),
            ..Default::default()
        }
    }
}
impl fmt::Display for Project {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\n        _________________________________
            ID: {}
            NAME: {}
            DESCRIPTION: {}
            COLOR: {}
            BACKEND TYPE: {}
            INBOX: {}
            TEAM INBOX: {}
            CHILD ORDER: {}
            DELETED: {}
            ARCHIVED: {}
            FAVORITE: {}
            SHARED: {}
            VIEW: {}
            SHOW COMPLETED: {}
            SORT ORDER: {}
            COLLAPSED: {}
            PARENT ID: {}
            SOURCE ID: {}\n        ---------------------------------        ",
            self.model.id.clone(),
            self.model.name.clone(),
            self.model.description.clone().unwrap_or_default(),
            self.model.color.clone().unwrap_or_default(),
            self.model.backend_type.clone().unwrap_or_default(),
            self.model.inbox_project.unwrap_or_default(),
            self.model.team_inbox.unwrap_or_default(),
            self.model.child_order.unwrap_or_default(),
            self.model.is_deleted,
            self.model.is_archived,
            self.model.is_favorite,
            self.model.shared.unwrap_or_default(),
            self.model.view_style.clone().unwrap_or_default(),
            self.model.show_completed.unwrap_or_default(),
            self.model.sort_order.unwrap_or_default(),
            self.model.collapsed,
            self.model.parent_id.clone().unwrap_or_default(),
            self.model.source_id.clone().unwrap_or_default()
        )
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
