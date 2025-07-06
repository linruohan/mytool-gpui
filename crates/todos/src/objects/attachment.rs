use super::BaseObject;
use crate::entity::prelude::AttachmentEntity;
use crate::entity::{AttachmentModel, ItemModel};
use crate::error::TodoError;
use crate::Store;
use sea_orm::{DatabaseConnection, EntityTrait};
use std::fmt;
use tokio::sync::OnceCell;

pub struct Attachment {
    pub model: AttachmentModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>, // 延迟初始化且线程安全
}
impl Attachment {
    pub fn id(&self) -> &str {
        &self.model.id
    }
    pub fn set_id(&mut self, id: String) -> &mut Self {
        self.model.id = id;
        self
    }
    pub fn item_id(&self) -> &str {
        &self.model.item_id
    }
    pub fn set_item_id(&mut self, id: String) -> &mut Self {
        self.model.item_id = id;
        self
    }
    pub fn file_type(&self) -> Option<String> {
        self.model.file_type.clone()
    }
    pub fn set_file_type(&mut self, file_type: String) -> &mut Self {
        self.model.file_type = Some(file_type);
        self
    }
    pub fn file_name(&self) -> &str {
        &self.model.file_name
    }
    pub fn set_file_name(&mut self, file_name: String) -> &mut Self {
        self.model.file_name = file_name;
        self
    }
    pub fn file_size(&self) -> u64 {
        self.model.file_size
    }
    pub fn set_file_size(&mut self, file_size: u64) -> &mut Self {
        self.model.file_size = file_size;
        self
    }
    pub fn file_path(&self) -> &str {
        &self.model.file_path
    }
    pub fn set_file_path(&mut self, file_path: String) -> &mut Self {
        self.model.file_path = file_path;
        self
    }
}

impl Attachment {
    pub fn new(db: DatabaseConnection, model: AttachmentModel,
    ) -> Attachment {
        let base = BaseObject::default();
        Self { model, base, db, store: OnceCell::new() }
    }
    pub async fn store(&self) -> &Store {
        self.store.get_or_init(|| async {
            Store::new(self.db.clone()).await
        }).await
    }
    pub async fn from_db(db: DatabaseConnection, attachment_id: &str) -> Result<Self, TodoError> {
        let attachment = AttachmentEntity::find_by_id(attachment_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", attachment_id)))?;

        Ok(Self::new(db, attachment))
    }

    pub async fn delete(&self) {
        self.store().await.delete_attachment(self.id());
    }
    pub async fn item(&self) -> Option<ItemModel> {
        self.store().await.get_item(&self.id()).await
    }
    pub fn set_item(&mut self, new_item_id: String) {
        self.model.item_id = new_item_id;
    }

    pub fn duplicate(&self) -> AttachmentModel {
        AttachmentModel {
            item_id: self.item_id().into(),
            file_type: None,
            file_name: self.file_name().into(),
            file_size: self.file_size().into(),
            file_path: self.file_path().into(),
            ..Default::default()
        }
    }
}
impl fmt::Display for Attachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "_________________________________\nID: {}\nITEM ID: {}\nFILE TYPE: {}\nFILE NAME: {}\nFILE SIZE: {}\nFILE PATH: {}\n---------------------------------",
            self.id().clone(),
            self.item_id(),
            self.file_type().clone().unwrap(),
            self.file_name(),
            self.file_size(),
            self.file_path(),
        )
    }
}
