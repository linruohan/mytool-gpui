use crate::entity::labels::ActiveModel as LabelActiveModel;
use crate::entity::labels::Model as LabelModel;
use crate::entity::prelude::LabelEntity;
use crate::entity::sources::Model as SourceModel;
use crate::enums::SourceType;
use crate::error::TodoError;
use crate::objects::BaseTrait;
use crate::BaseObject;
use crate::Store;
use crate::Util;
use sea_orm::prelude::*;
use sea_orm::Set;

#[derive(Clone, Debug)]
pub struct Label {
    pub model: LabelModel,
    base: BaseObject,
    store: Store,
    label_count: Option<usize>,
}

impl Label {
    pub fn name(&self) -> &str {
        &self.model.name
    }
    pub fn set_name(&mut self, name: impl Into<String>) -> &mut Self {
        self.model.name = name.into();
        self
    }
    pub fn color(&self) -> &str {
        &self.model.color
    }
    pub fn set_color(&mut self, color: impl Into<String>) -> &mut Self {
        self.model.color = color.into();
        self
    }
    pub fn item_order(&self) -> i32 {
        self.model.item_order
    }
    pub fn set_item_order(&mut self, order: i32) -> &mut Self {
        self.model.item_order = order;
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
    pub fn backend_type(&self) -> SourceType {
        self.model.backend_type
            .as_deref()
            .and_then(|b| serde_json::from_str(b).ok())
            .unwrap_or(SourceType::NONE)
    }
    pub fn set_backend_type(&mut self, backend_type: Option<String>) -> &mut Self {
        self.model.backend_type = backend_type;
        self
    }
    pub fn source_id(&self) -> String {
        self.model
            .source_id
            .as_deref().map_or_else(SourceType::LOCAL.to_string(), |id| id.to_string())
    }
    pub fn set_source_id(&mut self, source_id: Option<String>) -> &mut Self {
        self.model.source_id = source_id;
        self
    }
}

impl Label {
    pub fn new(db: DatabaseConnection, model: LabelModel) -> Self {
        let base = BaseObject::default();
        let store = Store::new(db);
        Self {
            model,
            base,
            store,
            label_count: None,
        }
    }
    pub async fn from_db(db: DatabaseConnection, label_id: &str) -> Result<Self, TodoError> {
        let label = LabelEntity::find_by_id(label_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Label {} not found", label_id)))?;

        Ok(Self::new(db, label))
    }


    pub async fn source_type(&self) -> SourceType {
        self.source()
            .await
            .ok()
            .and_then(|opt| opt.and_then(|s| serde_json::from_str(&s.source_type).ok()))
            .unwrap_or(SourceType::NONE)
    }
    pub async fn source(&self) -> Result<Option<SourceModel>, TodoError> {
        Ok(self.store.get_source(&self.source_id()).await?)
    }
    fn label_count(&mut self) -> usize {
        self.label_count = self.store.get_items_by_label(self.id(), false).len();
        self.label_count;
    }
    pub fn set_label_count(&mut self, count: usize) -> &mut Self {
        self.label_count = Some(count);
        self
    }

    pub fn short_name(&self) -> String {
        Util::get_default().get_short_name(self.name().clone(), 0)
    }
    pub fn delete_label(&self) {
        let items = self.store.get_items_by_label(self.id(), false);
        for item in items {
            item.delete_item_label(self.id());
        }
        self.store.delete_label(self.clone());
    }
}

impl BaseTrait for Label {
    fn id(&self) -> &str {
        &self.model.id
    }

    fn set_id(&mut self, id: &str) {
        self.model.id = id.into();
    }
}


// impl From<LabelModel> for Label {
//     fn from(model: LabelModel) -> Self {
//         Label {
//             model,
//             base: BaseObject::default(),
//             store: Store::default(),
//             label_count: None,
//         }
//     }
// }

impl Label {
    pub fn to_active_model(&self) -> LabelActiveModel {
        LabelActiveModel {
            id: self.id().into(),
            name: Set(self.name().to_string()),
            color: Set(self.color().to_string()),
            item_order: Set(self.item_order()),
            is_deleted: Set(self.is_deleted()),
            is_favorite: Set(self.is_favorite()),
            backend_type: Set(self.backend_type().map(|b| b.to_string())),
            source_id: Set(Some(self.source_id())),
            ..Default::default()
        }
    }
}