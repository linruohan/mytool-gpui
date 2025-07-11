use crate::entity::prelude::ProjectEntity;
use crate::entity::{ItemModel, ProjectModel, SectionModel, SourceModel};
use crate::enums::{ProjectIconStyle, ProjectViewStyle, SourceType};
use crate::error::TodoError;
use crate::objects::{BaseTrait, Source};
use crate::utils::Util;
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
    pub fn icon_style(&self) -> ProjectIconStyle {
        self.model
            .icon_style
            .as_deref()
            .map_or(ProjectIconStyle::PROGRESS, ProjectIconStyle::parse)
    }
    pub fn set_icon_style(&mut self, icon_style: ProjectIconStyle) -> &mut Self {
        self.model.icon_style = Some(icon_style.to_string());
        self
    }
    pub fn backend_type(&self) -> SourceType {
        self.model
            .backend_type
            .as_deref()
            .map_or(SourceType::NONE, SourceType::parse)
    }
    pub fn set_backend_type(&mut self, backend_type: SourceType) -> &mut Self {
        self.model.backend_type = Some(backend_type.to_string());
        self
    }
    pub fn view_style(&self) -> ProjectViewStyle {
        self.model
            .view_style
            .as_ref()
            .map_or(ProjectViewStyle::LIST, ProjectViewStyle::parse)
    }
    pub fn set_view_style(&mut self, view_style: ProjectViewStyle) -> &mut Self {
        self.model.view_style = Some(view_style.to_string());
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
    pub async fn source_type(&self) -> SourceType {
        if let Some(source_model) = self.source().await {
            if let Ok(source) = Source::from_db(self.db.clone(), &source_model.id).await {
                return source.source_type();
            }
        }
        SourceType::NONE
    }
    pub async fn source(&self) -> Option<SourceModel> {
        self.store()
            .await
            .get_source(&self.model.source_id.as_ref()?)
            .await
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
        self.model
            .parent_id
            .as_ref()
            .map_or_else(String::new, |id| id.to_string())
    }
    pub fn short_name(&self) -> String {
        Util::default().get_short_name(&self.model.name, 0)
    }

    pub fn is_inbox_project(&self) -> bool {
        // return id == Services.Settings.get_default ().settings.get_string ("local-inbox-project-id");
        self.model.id == "inbox"
    }

    pub async fn sections(&self) -> Vec<SectionModel> {
        self.store()
            .await
            .get_sections_by_project(&self.model.id)
            .await
    }

    pub async fn items(&self) -> Vec<ItemModel> {
        let mut items = self
            .store()
            .await
            .get_items_by_project(&self.model.id)
            .await;
        items.sort_by(|a, b| a.child_order.cmp(&b.child_order));
        items
    }
    pub async fn sections_archived(&self) -> Vec<SectionModel> {
        self.store()
            .await
            .get_sections_archived_by_project(&self.model.id)
            .await
    }

    pub async fn items_checked(&self) -> Vec<ItemModel> {
        self.store()
            .await
            .get_items_checked_by_project(&self.model.id)
            .await
    }
    pub async fn all_items(&self) -> Vec<ItemModel> {
        self.store()
            .await
            .get_items_by_project(&self.model.id)
            .await
    }
    pub async fn items_pinned(&self) -> Vec<ItemModel> {
        self.store()
            .await
            .get_items_by_project_pinned(&self.model.id)
            .await
    }
    pub async fn subprojects(&self) -> Vec<ProjectModel> {
        self.store().await.get_subprojects(&self.model.id).await
    }
    pub async fn parent(&self) -> Option<ProjectModel> {
        self.store()
            .await
            .get_project(self.model.parent_id.as_ref()?)
            .await
    }
    pub fn is_deck(&self) -> bool {
        self.model.id.contains("deck--board")
    }
    pub async fn project_count(&self) -> usize {
        self.items().await.len()
    }
    pub async fn update_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        self.store().await.update_project(project).await
    }
    pub async fn update(
        &self,
        use_timeout: bool,
        show_loading: bool,
    ) -> Result<ProjectModel, TodoError> {
        // if (update_timeout_id != 0) {
        //     GLib.Source.remove (update_timeout_id);
        // }
        //
        // uint timeout = Constants.UPDATE_TIMEOUT;
        // if (use_timeout) {
        //     timeout = 0;
        // }
        //
        // update_timeout_id = Timeout.add (timeout, () => {
        //     update_timeout_id = 0;
        //
        //     if (backend_type == SourceType.LOCAL) {
        //         Services.Store.instance ().update_project (this);
        //     } else if (backend_type == SourceType.TODOIST) {
        //         if (show_loading) {
        //             loading = true;
        //         }
        //
        //         Services.Todoist.get_default ().update.begin (this, (obj, res) => {
        //             Services.Todoist.get_default ().update.end (res);
        //             Services.Store.instance ().update_project (this);
        //             loading = false;
        //         });
        //     } else if (backend_type == SourceType.CALDAV) {
        //         if (show_loading) {
        //             loading = true;
        //         }
        //
        //         Services.CalDAV.Core.get_default ().update_tasklist.begin (this, (obj, res) => {
        //             Services.CalDAV.Core.get_default ().update_tasklist.end (res);
        //             Services.Store.instance ().update_project (this);
        //             loading = false;
        //         });
        //     }
        //
        //     return GLib.Source.REMOVE;
        // });
        todo!()
    }
    pub async fn get_subproject(&self, subproject_id: &str) -> Option<ProjectModel> {
        let subprojects = self.subprojects().await;
        subprojects.iter().find(|p| p.id == subproject_id).cloned()
    }
    pub async fn add_subproject_if_not_exists(
        &self,
        pro: &mut ProjectModel,
    ) -> Result<ProjectModel, TodoError> {
        let subproject = self.get_subproject(&pro.id).await;
        if subproject.is_none() {
            pro.parent_id = Some(self.model.id.clone());
            self.store().await.insert_project(pro.clone()).await
        } else {
            Ok(subproject.unwrap())
        }
    }
    pub fn set_parent(&mut self, parent: ProjectModel) {
        self.model.parent_id = Some(parent.id.clone());
    }

    pub(crate) fn update_count(&self) {
        todo!()
    }

    pub async fn add_subproject(
        &self,
        subproject: ProjectModel,
    ) -> Result<ProjectModel, TodoError> {
        self.store().await.insert_project(subproject).await
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
