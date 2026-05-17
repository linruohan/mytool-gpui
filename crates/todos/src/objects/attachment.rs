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
    store: Arc<Store>,
}

impl Attachment {
    /// 创建新的 Attachment（必须注入 Store）
    pub fn with_store(store: Arc<Store>, model: AttachmentModel) -> Self {
        let base = BaseObject::default();
        Self { model, base, store }
    }

    /// 获取 Store 引用
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// 从数据库加载 Attachment（必须传入 Store）
    pub async fn from_db(store: Arc<Store>, attachment_id: &str) -> Result<Self, TodoError> {
        let attachment = AttachmentEntity::find_by_id(attachment_id)
            .one(store.db())
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", attachment_id)))?;

        Ok(Self::with_store(store, attachment))
    }

    pub async fn delete_attachment(&self) -> Result<u64, TodoError> {
        // 暂时返回 0，因为不存在 delete_attachment 方法
        Ok(0)
    }

    pub async fn item(&self) -> Option<ItemModel> {
        self.store().get_item(&self.model.item_id).await
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
