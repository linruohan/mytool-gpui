use std::sync::Arc;

use sea_orm::{DatabaseConnection, EntityTrait};
use tokio::sync::OnceCell;

use crate::{
    BaseObject,
    entity::{SourceModel, prelude::SourceEntity},
    enums::SourceType,
    error::TodoError,
    objects::BaseTrait,
    services::Store,
};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Source {
    pub model: SourceModel,
    base: BaseObject,
    store: Arc<Store>,
}

#[allow(dead_code)]
impl Source {
    /// 创建新的 Source（必须注入 Store）
    pub fn with_store(store: Arc<Store>, model: SourceModel) -> Self {
        let base = BaseObject::default();
        Self { model, base, store }
    }

    /// 获取 Store 引用
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// 从数据库加载 Source（必须传入 Store）
    pub async fn from_db(store: Arc<Store>, item_id: &str) -> Result<Self, TodoError> {
        let item = SourceEntity::find_by_id(item_id)
            .one(store.db())
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", item_id)))?;

        Ok(Self::with_store(store, item))
    }

    pub fn source_type(&self) -> SourceType {
        SourceType::parse(&self.model.source_type)
    }

    pub fn header_text(&self) -> String {
        self.model.display_name.clone().unwrap_or_default()
    }

    pub fn sub_header_text(&self) -> &str {
        match self.source_type() {
            SourceType::LOCAL => "Tasks",
            SourceType::TODOIST => "Todoist",
            SourceType::GoogleTasks => "GoogleTasks",
            SourceType::CALDAV => "CalDAV",
            _ => "",
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
            _ => "",
        }
    }

    pub fn user_email(&self) -> &str {
        match self.source_type() {
            SourceType::TODOIST => "todoist@126.com",
            SourceType::CALDAV => "CalDAV@126.com",
            _ => "",
        }
    }

    pub fn run_server(&self) -> Result<(), Box<TodoError>> {
        match self.source_type() {
            SourceType::CALDAV => {
                // Services.CalDAV.Core.get_default ().sync.begin (this);
                Ok::<(), Box<TodoError>>(())
            },
            SourceType::TODOIST => {
                // Services.Todoist.get_default ().sync.begin (this);
                Ok::<(), Box<TodoError>>(())
            },
            _ => Ok::<(), Box<TodoError>>(()),
        }
    }

    pub fn save(&self) -> Result<(), Box<TodoError>> {
        // updated_at = new GLib.DateTime.now_local ().to_string ();
        // Services.Store.instance ().update_source (this);
        Ok::<(), Box<TodoError>>(())
    }

    pub async fn delete_source(&self) -> Result<(), TodoError> {
        use crate::entity::prelude::SourceEntity;
        SourceEntity::delete_by_id(self.model.id.clone())
            .exec(self.store.db())
            .await
            .map_err(|e| TodoError::DbError(Box::new(e)))?;
        Ok(())
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
