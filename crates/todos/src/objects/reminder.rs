use chrono::{Duration, NaiveDateTime};
use sea_orm::{DatabaseConnection, EntityTrait};
use tokio::sync::OnceCell;

use crate::{
    BaseObject, Store,
    entity::{ItemModel, ReminderModel, SourceModel, prelude::ReminderEntity},
    enums::ReminderType,
    error::TodoError,
    objects::{BaseTrait, DueDate, Item, Project},
    utils,
};

#[derive(Clone, Debug)]
pub struct Reminder {
    pub model: ReminderModel,
    base: BaseObject,
    db: DatabaseConnection,
    store: OnceCell<Store>,
}

impl Reminder {
    pub fn due(&self) -> Option<DueDate> {
        self.model
            .due
            .as_ref()
            .map(|json_str| serde_json::from_str::<DueDate>(json_str).ok())
            .unwrap_or_default()
    }

    pub fn set_due(&mut self, due: DueDate) -> &mut Self {
        self.model.due = Some(serde_json::value::to_value(due).unwrap().to_string());
        self
    }

    pub fn reminder_type(&self) -> ReminderType {
        self.model
            .reminder_type
            .as_ref()
            .and_then(|s| serde_json::from_str::<ReminderType>(s).ok())
            .unwrap_or(ReminderType::ABSOLUTE)
    }

    pub fn set_reminder_type(&mut self, reminder_type: &ReminderType) -> &mut Self {
        self.model.reminder_type = Some(reminder_type.to_string());
        self
    }
}

impl Reminder {
    pub fn new(db: DatabaseConnection, model: ReminderModel) -> Self {
        let base = BaseObject::default();
        Self { model, base, db, store: OnceCell::new() }
    }

    pub async fn store(&self) -> &Store {
        self.store.get_or_init(|| async { Store::new(self.db.clone()).await }).await
    }

    pub async fn from_db(db: DatabaseConnection, item_id: &str) -> Result<Self, TodoError> {
        let item = ReminderEntity::find_by_id(item_id)
            .one(&db)
            .await?
            .ok_or_else(|| TodoError::NotFound(format!("Item {} not found", item_id)))?;

        Ok(Self::new(db, item))
    }

    // generate_accessors!(reminder_type:Option<String>);

    pub async fn item(&self) -> Option<ItemModel> {
        self.store().await.get_item(&self.model.item_id.as_ref()?).await
    }

    pub async fn datetime(&self) -> Option<NaiveDateTime> {
        match self.reminder_type() {
            ReminderType::ABSOLUTE => self.due().as_ref()?.datetime(),
            _ => {
                let item_id = self.item().await?.id;
                let item = Item::from_db(self.db.clone(), &item_id).await.ok()?;
                item.due().as_ref()?.datetime().map(|dt| {
                    dt - Duration::minutes(self.model.mm_offset.unwrap_or_default() as i64)
                })
            },
        }
    }

    pub async fn relative_text(&self) -> String {
        match self.reminder_type() {
            ReminderType::ABSOLUTE => {
                let date_time =
                    self.due().as_ref().and_then(|due| due.datetime()).unwrap_or_default();
                utils::DateTime::default().get_relative_date_from_date(&date_time).to_string()
            },
            ReminderType::RELATIVE => utils::Util::get_default()
                .get_reminders_mm_offset_text(self.model.mm_offset.unwrap_or_default())
                .to_string(),

            _ => String::new(),
        }
    }

    pub async fn delete(&self) -> Result<u64, TodoError> {
        // if (item.project.source_type == SourceType.TODOIST) {
        //     loading = true;
        //     Services.Todoist.get_default ().delete.begin (this, (obj, res) => {
        //         if (Services.Todoist.get_default ().delete.end (res).status) {
        //             Services.Store.instance ().delete_reminder (this);
        //             loading = false;
        //         }
        //     });
        // } else {
        self.store().await.delete_reminder(&self.model.id).await
    }

    pub async fn source(&self) -> Option<SourceModel> {
        let item_id = self.item().await?.id;
        let item = Item::from_db(self.db.clone(), &item_id).await.ok()?;
        let project_id = item.project().await?.id;
        let project = Project::from_db(self.db.clone(), &project_id).await.ok()?;
        project.source().await
    }

    pub fn duplicate(&self) -> ReminderModel {
        ReminderModel {
            notify_uid: self.model.notify_uid,
            service: self.model.service.clone(),
            due: self.model.due.clone(),
            mm_offset: self.model.mm_offset,
            ..Default::default()
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
