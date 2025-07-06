use crate::entity::prelude::ReminderEntity;
use crate::entity::ReminderModel;
use crate::enums::ReminderType;
use crate::error::TodoError;
use crate::generate_accessors;
use crate::objects::{BaseTrait, DueDate, ToBool};
use crate::utils;
use crate::BaseObject;
use crate::Item;
use crate::Source;
use crate::Store;
use chrono::Duration;
use chrono::NaiveDateTime;
use sea_orm::{DatabaseConnection, EntityTrait};
use tokio::sync::OnceCell;

#[derive(Clone, Debug)]
pub struct Reminder {
    pub model: ReminderModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>,

}

impl Reminder {
    pub fn new(db: DatabaseConnection, model: ReminderModel) -> Self {
        let base = BaseObject::default();
        Self { model, base, db, store: OnceCell::new() }
    }

    pub async fn store(&self) -> &Store {
        self.store.get_or_init(|| async {
            Store::new(self.db.clone()).await
        }).await
    }
    pub async fn from_db(db: DatabaseConnection, item_id: &str) -> Result<Self, TodoError> {
        let item = ReminderEntity::find_by_id(item_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", item_id)))?;

        Ok(Self::new(db, item))
    }
    generate_accessors!(item_id:Option<String>);
    generate_accessors!(notify_uid:Option<i32>);
    generate_accessors!(service:Option<String>);
    // generate_accessors!(reminder_type:Option<String>);
    pub fn reminder_type(&self) -> ReminderType {
        self.reminder_type
            .as_ref()
            .and_then(|s| serde_json::from_str::<ReminderType>(s).ok())
            .unwrap_or(ReminderType::ABSOLUTE)
    }
    pub fn set_reminder_type(&mut self, reminder_type: &ReminderType) {
        self.reminder_type = Some(reminder_type.to_string());
    }
    generate_accessors!(@due due:Option<String>);
    generate_accessors!(mm_offset:Option<i32>);
    generate_accessors!(@bool is_deleted:Option<i32>);

    pub fn item(&self) -> Option<Item> {
        self.item_id
            .as_ref()
            .and_then(|id| Store::instance().get_item(id))
    }
    pub fn relative_text(&self) -> String {
        match self.reminder_type() {
            ReminderType::ABSOLUTE => self
                .due()
                .datetime()
                .map(|dt| utils::DateTime::default().get_relative_date_from_date(&dt))
                .unwrap_or_default(),
            ReminderType::RELATIVE => utils::Util::get_default()
                .get_reminders_mm_offset_text(self.mm_offset.unwrap_or(0))
                .to_string(),

            _ => String::new(),
        }
    }
    pub fn datetime(&self) -> Option<NaiveDateTime> {
        match self.reminder_type() {
            ReminderType::ABSOLUTE => self.due().datetime(),
            _ => self.item()?.due().datetime().map(|dt| {
                let offset = -self.mm_offset.unwrap_or(0);
                let duration = Duration::minutes(offset as i64);
                dt + duration
            }),
        }
    }
    fn source(&self) -> Option<Source> {
        self.item()
            .and_then(|i| i.project().and_then(|p| p.source()))
    }
    pub fn delete(&self) {
        // if (item.project.source_type == SourceType.TODOIST) {
        //     loading = true;
        //     Services.Todoist.get_default ().delete.begin (this, (obj, res) => {
        //         if (Services.Todoist.get_default ().delete.end (res).status) {
        //             Services.Store.instance ().delete_reminder (this);
        //             loading = false;
        //         }
        //     });
        // } else {
        Store::instance().delete_reminder(self);
    }
    pub fn duplicate(&self) -> Reminder {
        Self {
            notify_uid: self.notify_uid,
            service: self.service.clone(),
            due: self.due.clone(),
            mm_offset: self.mm_offset,
            ..self.clone()
        }
    }
}

impl BaseTrait for Reminder {
    fn id(&self) -> &str {
        &self.model.id
    }

    fn set_id(&mut self, id: &str) {
        self.model.id = id.into();
    }
}

