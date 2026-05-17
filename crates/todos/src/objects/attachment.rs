use std::{fmt, sync::Arc};

use sea_orm::{DatabaseConnection, EntityTrait};
use tokio::sync::OnceCell;

use super::BaseObject;
use crate::{
    Store,
    entity::{AttachmentModel, ItemModel, prelude::AttachmentEntity},
    error::TodoError,
};

pub struct Attachment {
    pub model: AttachmentModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Arc<Store>>,
}

impl Attachment {
    /// 创建新的 Attachment（懒加载 Store）
    pub fn new(db: DatabaseConnection, model: AttachmentModel) -> Attachment {
        let base = BaseObject::default();
        Self { model, base, db, store: OnceCell::new() }
    }

    /// 创建新的 Attachment（注入 Store，推荐）
    pub fn with_store(store: Arc<Store>, model: AttachmentModel) -> Self {
        let base = BaseObject::default();
        let db = store.db().clone();
        let store_cell = OnceCell::new();
        store_cell.set(store).expect("Store already initialized");
        Self { model, base, db, store: store_cell }
    }

    pub async fn store(&self) -> &Store {
        self.store
            .get_or_init(|| async {
                Arc::new(Store::new(self.db.clone()).await.expect(
                    "Failed to initialize Store for Attachment: database connection failed",
                ))
            })
            .await
    }

    pub async fn from_db(db: DatabaseConnection, attachment_id: &str) -> Result<Self, TodoError> {
        let attachment = AttachmentEntity::find_by_id(attachment_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", attachment_id)))?;

        Ok(Self::new(db, attachment))
    }

    pub async fn delete_attachment(&self) -> Result<u64, TodoError> {
        // 暂时返回 0，因为不存在 delete_attachment 方法
        Ok(0)
    }

    pub async fn item(&self) -> Option<ItemModel> {
        self.store().await.get_item(&self.model.item_id).await
    }

    pub fn set_item(&mut self, new_item_id: &str) -> &mut Self {
        self.model.item_id = new_item_id.to_string();
        self
    }

    pub fn duplicate(&self) -> AttachmentModel {
        AttachmentModel {
            file_type: self.model.file_type.clone(),
            file_name: self.model.file_name.clone(),
            file_size: self.model.file_size,
            file_path: self.model.file_path.clone(),
            ..Default::default()
        }
    }
}
impl fmt::Display for Attachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "_________________________________\nID: {}\nITEM ID: {}\nFILE TYPE: {}\nFILE NAME: \
             {}\nFILE SIZE: {}\nFILE PATH: {}\n---------------------------------",
            self.model.id.clone(),
            self.model.item_id.clone(),
            self.model.file_type.as_ref().unwrap_or(&"".to_string()),
            self.model.file_name.clone(),
            self.model.file_size.clone(),
            self.model.file_path.clone(),
        )
    }
}
