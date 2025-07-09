use crate::entity::prelude::SourceEntity;
use crate::entity::SourceModel;
use crate::enums::SourceType;
use crate::error::TodoError;
use crate::objects::BaseTrait;
use crate::services::Store;
use crate::BaseObject;
use sea_orm::{DatabaseConnection, EntityTrait};
use tokio::sync::OnceCell;

#[derive(Clone, Debug)]
pub struct Source {
    pub model: SourceModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>,
}

impl Source {
    pub fn new(db: DatabaseConnection, model: SourceModel) -> Self {
        let base = BaseObject::default();
        Self {
            model,
            base,
            db,
            store: OnceCell::new(),
        }
    }

    pub async fn store(&self) -> &Store {
        self.store
            .get_or_init(|| async { Store::new(self.db.clone()).await })
            .await
    }
    pub async fn from_db(db: DatabaseConnection, item_id: &str) -> Result<Self, TodoError> {
        let item = SourceEntity::find_by_id(item_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", item_id)))?;

        Ok(Self::new(db, item))
    }
    pub fn source_type(&self) -> SourceType {
        SourceType::parse(Some(&self.model.source_type))
    }
    pub fn header_text(&self) -> String {
        self.model.display_name.clone().unwrap_or_default()
    }
    pub fn sub_header_text(&self) -> &str {
        match self.source_type() {
            SourceType::LOCAL => { "Tasks" }
            SourceType::TODOIST => { "Todoist" }
            SourceType::GoogleTasks => { "GoogleTasks" }
            SourceType::CALDAV => { "CalDAV" }
            _ => ""
        }
    }
    pub fn avatar_path(&self) -> &str {
        match self.source_type() {
            SourceType::LOCAL => "assets/images/local.png",
            SourceType::TODOIST => "assets/images/todoist.png",
            SourceType::GoogleTasks => "assets/images/google_tasks.png",
            SourceType::CALDAV => "assets/images/caldav.png",
            _ => "assets/images/default.png",
        }
    }
    pub fn user_displayname(&self) -> &str {
        match self.source_type() {
            SourceType::TODOIST => "Todoist",
            SourceType::CALDAV => "CALDAV",
            _ => ""
        }
    }
    pub fn user_email(&self) -> &str {
        match self.source_type() {
            SourceType::TODOIST => "todoist@126.com",
            SourceType::CALDAV => "CalDAV@126.com",
            _ => ""
        }
    }
    pub fn run_server(&self) -> Result<(), TodoError> {
        match self.source_type() {
            SourceType::CALDAV => {
                // Services.CalDAV.Core.get_default ().sync.begin (this);
                Ok(())
            }
            SourceType::TODOIST => {
                // Services.Todoist.get_default ().sync.begin (this);
                Ok(())
            }
            _ => Ok(())
        }
    }
    pub fn save(&self) -> Result<(), TodoError> {
        // updated_at = new GLib.DateTime.now_local ().to_string ();
        // Services.Store.instance ().update_source (this);
        Ok(())
    }
    pub async fn delete_source(&self) -> Result<u64, TodoError> {
        self.store().await.delete_source(&self.model.id).await
    }
}

impl BaseTrait for Source {
    fn id(&self) -> &str {
        &self.model.id
    }

    fn set_id(&mut self, id: &str) {
        self.model.id = id.into();
    }
}
