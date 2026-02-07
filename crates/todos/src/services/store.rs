use chrono::{Datelike, NaiveDateTime, Utc};
use futures::stream::{self, StreamExt};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set, prelude::Expr,
};

use crate::{
    constants,
    entity::{
        AttachmentActiveModel, AttachmentModel, ItemActiveModel, ItemModel, LabelActiveModel,
        LabelModel, ProjectActiveModel, ProjectModel, ReminderActiveModel, ReminderModel,
        SectionActiveModel, SectionModel, SourceActiveModel, SourceModel, attachments, items,
        labels, prelude::*, projects, reminders, sections,
    },
    error::TodoError,
    objects::{BaseTrait, Item, Section},
    services::EventBus,
    utils::DateTime,
};

#[derive(Clone, Debug)]
pub struct Store {
    db: DatabaseConnection,
    event_bus: EventBus,
}

impl Store {
    pub async fn new(db: DatabaseConnection) -> Store {
        Self { db, event_bus: EventBus::new() }
    }

    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    // attachments
    pub async fn attachments(&self) -> Vec<AttachmentModel> {
        AttachmentEntity::find().all(&self.db).await.unwrap_or_default()
    }

    pub async fn delete_attachment(&self, id: &str) -> Result<u64, TodoError> {
        Ok(AttachmentEntity::delete_by_id(id).exec(&self.db).await?.rows_affected)
    }

    pub async fn insert_attachment(
        &self,
        attachments: AttachmentModel,
    ) -> Result<AttachmentModel, TodoError> {
        let mut active_attachment: AttachmentActiveModel = attachments.into();
        Ok(active_attachment.insert(&self.db).await?)
    }

    pub async fn get_attachments_by_itemid(&self, item_id: &str) -> Vec<AttachmentModel> {
        AttachmentEntity::find()
            .filter(attachments::Column::ItemId.eq(item_id))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    // sources
    pub async fn sources(&self) -> Vec<SourceModel> {
        SourceEntity::find().all(&self.db).await.unwrap_or_default()
    }

    pub async fn get_source(&self, id: &str) -> Option<SourceModel> {
        SourceEntity::find_by_id(id).one(&self.db).await.unwrap_or_default()
    }

    pub async fn insert_source(&self, sources: SourceModel) -> Result<SourceModel, TodoError> {
        let mut active_source: SourceActiveModel = sources.into();
        Ok(active_source.insert(&self.db).await?)
    }

    pub async fn delete_source(&self, source_id: &str) -> Result<u64, TodoError> {
        let result = SourceEntity::delete_by_id(source_id).exec(&self.db).await?;
        if result.rows_affected > 0 {
            for project in self.get_projects_by_source(source_id).await {
                self.delete_project(&project.id).await?;
            }
        }
        Ok(1)
    }

    pub async fn update_source(&self, source: SourceModel) -> Result<SourceModel, TodoError> {
        let mut active_source: SourceActiveModel =
            <SourceModel as Into<SourceActiveModel>>::into(source).reset_all();
        Ok(active_source.update(&self.db).await?)
    }

    // projects
    pub async fn projects(&self) -> Vec<ProjectModel> {
        ProjectEntity::find().all(&self.db).await.unwrap_or_default()
    }

    pub async fn insert_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        let mut active_project: ProjectActiveModel = project.into();
        Ok(active_project.insert(&self.db).await?)
    }

    pub async fn get_project(&self, id: &str) -> Option<ProjectModel> {
        ProjectEntity::find_by_id(id).one(&self.db).await.unwrap_or_default()
    }

    pub async fn get_projects_by_source(&self, id: &str) -> Vec<ProjectModel> {
        ProjectEntity::find()
            .filter(projects::Column::SourceId.eq(id))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn update_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        let mut active_project: ProjectActiveModel =
            <ProjectModel as Into<ProjectActiveModel>>::into(project).reset_all();
        Ok(active_project.update(&self.db).await?)
    }

    pub async fn delete_project(&self, id: &str) -> Result<(), TodoError> {
        Box::pin(async move {
            ProjectEntity::delete_by_id(id).exec(&self.db).await?;
            self.delete_project(id).await?;
            Ok(())
        })
        .await
    }

    async fn _delete_project(&self, project_id: &str) -> Result<(), TodoError> {
        for section in self.get_sections_by_project(project_id).await {
            self.delete_section(&section.id).await?;
        }
        for item in self.get_items_by_project(project_id).await {
            self.delete_item(&item.id).await?;
        }
        for subproject in self.get_subprojects(project_id).await {
            self.delete_project(&subproject.id).await?;
        }
        Ok(())
    }

    pub async fn update_project_id(&self, project_id: &str, new_id: &str) -> Result<(), TodoError> {
        let project = ProjectEntity::find_by_id(project_id)
            .one(&self.db)
            .await?
            .ok_or(TodoError::NotFound("project not found".to_string()))?;
        ProjectEntity::update(ProjectActiveModel {
            id: Set(new_id.to_string()),
            is_archived: Set(true),
            ..project.into()
        })
        .exec(&self.db)
        .await?;
        SectionEntity::update_many()
            .col_expr(sections::Column::ProjectId, Expr::value(new_id.to_string()))
            .filter(sections::Column::ProjectId.eq(project_id))
            .exec(&self.db)
            .await?;
        ItemEntity::update_many()
            .col_expr(items::Column::ProjectId, Expr::value(new_id.to_string()))
            .filter(items::Column::ProjectId.eq(project_id))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    pub async fn next_project_child_order(&self, source_id: &str) -> i32 {
        ProjectEntity::find()
            .filter(
                projects::Column::SourceId
                    .eq(source_id)
                    .and(projects::Column::IsDeleted.eq(0).and(projects::Column::IsArchived.eq(0))),
            )
            .count(&self.db)
            .await
            .unwrap_or(0) as i32
    }

    pub async fn archive_project(&self, project_id: &str) -> Result<(), TodoError> {
        let mut project = ProjectEntity::find_by_id(project_id)
            .one(&self.db)
            .await?
            .ok_or(TodoError::NotFound("project not found".to_string()))?;
        project.is_archived = !project.is_archived;
        ProjectEntity::update(ProjectActiveModel {
            id: Set(project_id.to_string()),
            ..project.into()
        })
        .exec(&self.db)
        .await?;

        let items = self.get_items_by_project(project_id).await;
        for item in items {
            self.archive_item(&item.id, true).await?;
        }

        let sections = self.get_sections_by_project(project_id).await;
        for section in sections {
            self.archive_section(&section.id, true).await?;
        }
        Ok(())
    }

    pub async fn get_subprojects(&self, id: &str) -> Vec<ProjectModel> {
        ProjectEntity::find()
            .filter(projects::Column::ParentId.eq(id))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_inbox_project(&self) -> Vec<ProjectModel> {
        ProjectEntity::find()
            .filter(projects::Column::Id.eq(constants::INBOX_PROJECT_ID))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_all_projects_archived(&self) -> Vec<ProjectModel> {
        ProjectEntity::find()
            .filter(projects::Column::IsArchived.eq(1))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_all_projects_by_search(&self, search_text: &str) -> Vec<ProjectModel> {
        let search_lover = search_text.to_lowercase();
        ProjectEntity::find()
            .filter(projects::Column::Name.contains(&search_lover))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    // // sections
    pub async fn sections(&self) -> Vec<SectionModel> {
        SectionEntity::find().all(&self.db).await.unwrap_or_default()
    }

    pub async fn get_section(&self, id: &str) -> Option<SectionModel> {
        SectionEntity::find_by_id(id).one(&self.db).await.unwrap_or_default()
    }

    pub async fn get_sections_by_project(&self, project_id: &str) -> Vec<SectionModel> {
        SectionEntity::find()
            .filter(sections::Column::ProjectId.eq(project_id))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_sections_archived_by_project(&self, project_id: &str) -> Vec<SectionModel> {
        let sections_model = match SectionEntity::find()
            .filter(sections::Column::ProjectId.eq(project_id))
            .all(&self.db)
            .await
        {
            Ok(sections) => sections,
            Err(_) => return vec![],
        };
        stream::iter(sections_model)
            .filter_map(|model| async move {
                if let Ok(section) = Section::from_db(self.db.clone(), &model.id).await
                    && section.was_archived().await
                {
                    return Some(model);
                };
                None
            })
            .collect()
            .await
    }

    pub async fn get_all_sections_by_search(&self, search_text: &str) -> Vec<SectionModel> {
        let search_lover = search_text.to_lowercase();
        SectionEntity::find()
            .filter(sections::Column::Name.contains(&search_lover))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn update_section(&self, section: SectionModel) -> Result<SectionModel, TodoError> {
        let mut active_section: SectionActiveModel =
            <SectionModel as Into<SectionActiveModel>>::into(section).reset_all();
        Ok(active_section.update(&self.db).await?)
    }

    pub async fn move_section(&self, section_id: &str, project_id: &str) -> Result<(), TodoError> {
        let section = SectionEntity::find_by_id(section_id)
            .one(&self.db)
            .await?
            .ok_or(TodoError::NotFound("section not found".to_string()))?;
        SectionActiveModel {
            id: Set(section_id.to_string()),
            project_id: Set(Some(project_id.to_string())),
            ..section.into()
        }
        .update(&self.db)
        .await?;
        ItemEntity::update_many()
            .col_expr(items::Column::ProjectId, Expr::value(project_id.to_string()))
            .filter(items::Column::SectionId.eq(section_id))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    pub async fn update_section_id(&self, section_id: &str, new_id: &str) -> Result<(), TodoError> {
        let section = SectionEntity::find_by_id(section_id)
            .one(&self.db)
            .await?
            .ok_or(TodoError::NotFound("section not found".to_string()))?;
        SectionActiveModel { id: Set(new_id.to_string()), ..section.into() }
            .update(&self.db)
            .await?;
        ItemEntity::update_many()
            .col_expr(items::Column::SectionId, Expr::value(new_id.to_string()))
            .filter(items::Column::SectionId.eq(section_id))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    pub async fn archive_section(&self, section_id: &str, archived: bool) -> Result<(), TodoError> {
        let section = SectionEntity::find_by_id(section_id)
            .one(&self.db)
            .await?
            .ok_or(TodoError::NotFound("section not found".to_string()))?;
        let archived_new = if section.is_archived == archived { !archived } else { archived };
        let active_model = SectionActiveModel {
            is_archived: Set(archived_new),
            archived_at: Set(Some(Utc::now().naive_utc())),
            ..section.into()
        };
        active_model.update(&self.db).await?;
        for item in self.get_items_by_section(section_id).await {
            self.archive_item(&item.id, true).await?;
        }
        Ok(())
    }

    pub async fn insert_section(&self, section: SectionModel) -> Result<SectionModel, TodoError> {
        let mut active_section: sections::ActiveModel = section.into();
        Ok(active_section.insert(&self.db).await?)
    }

    pub async fn delete_section(&self, section_id: &str) -> Result<(), TodoError> {
        let result = SectionEntity::delete_by_id(section_id).exec(&self.db).await?;
        if result.rows_affected > 0 {
            let items = ItemEntity::find()
                .filter(items::Column::SectionId.eq(section_id))
                .all(&self.db)
                .await?;
            for item in items {
                self.delete_item(&item.id).await?;
            }
        }
        Ok(())
    }

    // // items
    pub async fn items(&self) -> Vec<ItemModel> {
        ItemEntity::find().all(&self.db).await.unwrap_or_default()
    }

    pub async fn insert_item(&self, item: ItemModel, insert: bool) -> Result<ItemModel, TodoError> {
        let mut active_model: ItemActiveModel = item.into();
        let item_model = active_model.insert(&self.db).await?;
        self.add_item(item_model.clone(), insert);
        self.event_bus
            .publish(crate::services::event_bus::Event::ItemCreated(item_model.id.clone()));
        Ok(item_model)
    }

    pub async fn add_item(&self, item: ItemModel, insert: bool) {
        // Publish event for items position update
        if let Some(project_id) = &item.project_id
            && let Some(section_id) = &item.section_id
        {
            self.event_bus.publish(crate::services::event_bus::Event::ItemsPositionUpdated(
                project_id.clone(),
                section_id.clone(),
            ));
        }
    }

    pub async fn update_item(
        &self,
        item: ItemModel,
        update_id: &str,
    ) -> Result<ItemModel, TodoError> {
        let item_id = item.id.clone();
        let mut active_model: ItemActiveModel =
            <ItemModel as Into<ItemActiveModel>>::into(item).reset_all();
        let result = active_model.update(&self.db).await?;
        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id));
        Ok(result)
    }

    pub async fn update_item_pin(&self, item_id: &str, pinned: bool) -> Result<(), TodoError> {
        let item_model = self
            .get_item(item_id)
            .await
            .ok_or_else(|| TodoError::NotFound("item not found".to_string()))?;
        ItemEntity::update(ItemActiveModel { pinned: Set(pinned), ..item_model.into() })
            .exec(&self.db)
            .await?;
        Ok(())
    }

    pub async fn move_item(
        &self,
        item_id: &str,
        project_id: &str,
        section_id: &str,
    ) -> Result<(), TodoError> {
        let item_model = self
            .get_item(item_id)
            .await
            .ok_or_else(|| TodoError::NotFound("item not found".to_string()))?;
        ItemEntity::update(ItemActiveModel {
            id: Set(item_id.to_string()),
            project_id: Set(Some(project_id.to_string())),
            section_id: Set(Some(section_id.to_string())),
            ..item_model.into()
        })
        .exec(&self.db)
        .await?;
        let subitems = self.get_subitems(item_id).await;
        ItemEntity::update_many()
            .col_expr(items::Column::ProjectId, Expr::value(project_id.to_string()))
            .col_expr(items::Column::SectionId, Expr::value(section_id.to_string()))
            .filter(items::Column::ParentId.eq(item_id.to_string()))
            .exec(&self.db)
            .await?;

        // Publish events
        self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id.to_string()));
        self.event_bus.publish(crate::services::event_bus::Event::ItemsPositionUpdated(
            project_id.to_string(),
            section_id.to_string(),
        ));

        Ok(())
    }

    pub async fn delete_item(&self, item_id: &str) -> Result<(), TodoError> {
        let item_id_clone = item_id.to_string();
        Box::pin(async move {
            let result = ItemEntity::delete_by_id(item_id).exec(&self.db).await?;
            let mut subitems = ItemEntity::find()
                .filter(items::Column::ParentId.eq(item_id))
                .all(&self.db)
                .await?;
            for item in subitems {
                self.delete_item(&item.id).await?
            }
            self.event_bus.publish(crate::services::event_bus::Event::ItemDeleted(item_id_clone));
            Ok(())
        })
        .await
    }

    pub async fn archive_item(&self, item_id: &str, archived: bool) -> Result<(), TodoError> {
        let item_id_clone = item_id.to_string();
        Box::pin(async move {
            let item = Item::from_db(self.db.clone(), item_id).await?;
            if archived {
                item.archived();
            } else {
                item.unarchived();
            };
            let mut subitems = ItemEntity::find()
                .filter(items::Column::ParentId.eq(item_id))
                .all(&self.db)
                .await?;
            for item in subitems {
                self.archive_item(&item.id, archived).await?
            }
            // Publish event
            self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id_clone));
            Ok(())
        })
        .await
    }

    pub async fn complete_item(
        &self,
        item_id: &str,
        checked: bool,
        complete_subitems: bool,
    ) -> Result<(), TodoError> {
        let item_id_clone = item_id.to_string();
        Box::pin(async move {
            let active_model = ItemActiveModel {
                id: Set(item_id.to_string()),
                checked: Set(checked),
                completed_at: Set(Some(Utc::now().naive_utc())),
                ..ItemEntity::find_by_id(item_id)
                    .one(&self.db)
                    .await?
                    .ok_or(TodoError::NotFound("item not found".to_string()))?
                    .into()
            };
            let item_model = active_model.update(&self.db).await?;
            if complete_subitems {
                let mut subitems = ItemEntity::find()
                    .filter(items::Column::ParentId.eq(item_id))
                    .all(&self.db)
                    .await?;
                for item in subitems {
                    self.complete_item(&item.id, item_model.checked, complete_subitems).await?
                }
            };
            if let Some(parent) =
                ItemEntity::find().filter(items::Column::ParentId.eq(item_id)).one(&self.db).await?
            {
                self.complete_item(&parent.id, item_model.checked, false).await?
            };
            // Publish event
            self.event_bus.publish(crate::services::event_bus::Event::ItemUpdated(item_id_clone));
            Ok(())
        })
        .await
    }

    pub async fn update_item_id(&self, item_id: &str, new_id: &str) -> Result<(), TodoError> {
        // 更新item的id为新的id
        let item_model = ItemActiveModel {
            id: Set(new_id.to_string()),
            ..ItemEntity::find_by_id(item_id)
                .one(&self.db)
                .await?
                .ok_or(TodoError::NotFound("item not found".to_string()))?
                .into()
        };
        item_model.update(&self.db).await?;
        // 更新item的subitems的parent_id
        ItemEntity::update_many()
            .col_expr(items::Column::ParentId, Expr::value(new_id.to_string()))
            .filter(items::Column::ParentId.eq(item_id))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    pub async fn next_item_child_order(&self, project_id: &str, section_id: &str) -> i32 {
        ItemEntity::find()
            .filter(
                items::Column::ProjectId
                    .eq(project_id)
                    .and(items::Column::SectionId.eq(section_id)),
            )
            .count(&self.db)
            .await
            .unwrap_or(0) as i32
    }

    pub async fn get_item(&self, id: &str) -> Option<ItemModel> {
        ItemEntity::find_by_id(id).one(&self.db).await.unwrap_or_default()
    }

    pub async fn get_items_by_section(&self, section_id: &str) -> Vec<ItemModel> {
        ItemEntity::find()
            .filter(items::Column::SectionId.eq(section_id))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_subitems(&self, item_id: &str) -> Vec<ItemModel> {
        ItemEntity::find()
            .filter(items::Column::ParentId.eq(item_id))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_items_completed(&self) -> Vec<ItemModel> {
        let items_model =
            match ItemEntity::find().filter(items::Column::Checked.eq(1)).all(&self.db).await {
                Ok(items) => items,
                Err(_) => return vec![],
            };
        stream::iter(items_model)
            .filter_map(|model| async move {
                let item = Item::from_db(self.db.clone(), &model.id).await;
                if let Ok(item) = item
                    && !item.was_archived().await
                {
                    return Some(model);
                }
                None
            })
            .collect()
            .await
    }

    pub async fn get_item_by_ics(&self, ics: &str) -> Option<ItemModel> {
        ItemEntity::find().filter(items::Column::Id.eq(ics)).one(&self.db).await.unwrap_or_default()
    }

    pub async fn get_items_has_labels(&self) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find()
            .filter(items::Column::Labels.is_not_null())
            .all(&self.db)
            .await
        {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model)
            .filter_map(|model| async move {
                let item = Item::from_db(self.db.clone(), &model.id).await;
                if let Ok(item) = item
                    && item.has_labels().await
                    && item.model.checked
                    && !item.was_archived().await
                {
                    return Some(model);
                }
                None
            })
            .collect()
            .await
    }

    pub async fn get_items_by_label(&self, label_id: &str, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find()
            .filter(items::Column::Labels.is_not_null().and(items::Column::Checked.eq(1)))
            .all(&self.db)
            .await
        {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model)
            .filter_map(|model| async move {
                let item = Item::from_db(self.db.clone(), &model.id).await;
                if let Ok(item) = item
                    && item.has_label(label_id).await
                    && model.checked == checked
                    && !item.was_archived().await
                {
                    return Some(model);
                }
                None
            })
            .collect()
            .await
    }

    pub async fn get_items_checked(&self) -> Result<Vec<ItemModel>, TodoError> {
        Ok(ItemEntity::find().filter(items::Column::Checked.eq(1)).all(&self.db).await?)
    }

    pub async fn get_items_unchecked(&self) -> Result<Vec<ItemModel>, TodoError> {
        Ok(ItemEntity::find().filter(items::Column::Checked.eq(0)).all(&self.db).await?)
    }

    pub async fn get_items_checked_by_project(&self, project_id: &str) -> Vec<ItemModel> {
        ItemEntity::find()
            .filter(items::Column::ProjectId.eq(project_id).and(items::Column::Checked.eq(1)))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_subitems_uncomplete(&self, item_id: &str) -> Vec<ItemModel> {
        ItemEntity::find()
            .filter(items::Column::ParentId.eq(item_id).and(items::Column::Checked.eq(0)))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_items_by_project(&self, project_id: &str) -> Vec<ItemModel> {
        ItemEntity::find()
            .filter(items::Column::ProjectId.eq(project_id))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_items_by_project_pinned(&self, project_id: &str) -> Vec<ItemModel> {
        ItemEntity::find()
            .filter(items::Column::ProjectId.eq(project_id).and(items::Column::Pinned.eq(1)))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_items_by_date(&self, date: &NaiveDateTime, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find()
            .filter(items::Column::Pinned.eq(1).and(items::Column::Checked.eq(checked)))
            .all(&self.db)
            .await
        {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model)
            .filter_map(|model| async move {
                self.valid_item_by_date(&model.id, date, checked).await.then_some(model)
            })
            .collect()
            .await
    }

    pub async fn get_items_no_date(&self, checked: bool) -> Vec<ItemModel> {
        ItemEntity::find()
            .filter(items::Column::Due.is_null().and(items::Column::Checked.eq(checked)))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_items_repeating(&self, checked: bool) -> Vec<ItemModel> {
        ItemEntity::find()
            .filter(items::Column::Due.is_not_null().and(items::Column::Checked.eq(checked)))
            .all(&self.db)
            .await
            .unwrap_or_default()
        //     i.has_due() && i.due().is_recurring && i.checked() == checked && !i.was_archived()
    }

    pub async fn get_items_by_date_range(
        &self,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        checked: bool,
    ) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find()
            .filter(items::Column::Pinned.eq(1).and(items::Column::Checked.eq(checked)))
            .all(&self.db)
            .await
        {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model)
            .filter_map(|model| async move {
                self.valid_item_by_date_range(&model.id, start_date, end_date, checked)
                    .await
                    .then_some(model)
            })
            .collect()
            .await
    }

    pub async fn get_items_by_month(&self, date: &NaiveDateTime, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find()
            .filter(items::Column::Pinned.eq(1).and(items::Column::Checked.eq(checked)))
            .all(&self.db)
            .await
        {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model)
            .filter_map(|model| async move {
                self.valid_item_by_month(&model.id, date, checked).await.then_some(model)
            })
            .collect()
            .await
    }

    pub async fn get_items_pinned(&self, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find()
            .filter(items::Column::Pinned.eq(1).and(items::Column::Checked.eq(checked)))
            .all(&self.db)
            .await
        {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model)
            .filter_map(|model| async move {
                let Ok(item) = Item::from_db(self.db.clone(), &model.id).await else {
                    return None;
                };
                if item.was_archived().await {
                    return None;
                }
                Some(model)
            })
            .collect()
            .await
    }

    pub async fn get_items_by_priority(&self, priority: i32, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find()
            .filter(items::Column::Priority.eq(priority).and(items::Column::Checked.eq(checked)))
            .all(&self.db)
            .await
        {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model)
            .filter_map(|model| async move {
                let Ok(item) = Item::from_db(self.db.clone(), &model.id).await else {
                    return None;
                };
                if item.was_archived().await {
                    return None;
                }
                Some(model)
            })
            .collect()
            .await
    }

    pub async fn get_items_by_scheduled(&self, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find()
            .filter(items::Column::Due.is_not_null().and(items::Column::Checked.eq(checked)))
            .all(&self.db)
            .await
        {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model)
            .filter_map(|model| async move {
                let Ok(item) = Item::from_db(self.db.clone(), &model.id).await else {
                    return None;
                };
                if item.was_archived().await {
                    return None;
                }
                let now = Utc::now().naive_utc();
                // 检查截止日期
                item.due()
                    .and_then(|d| d.datetime())
                    .map(|due| due > now)
                    .unwrap_or(false)
                    .then_some(model)
            })
            .collect()
            .await
    }

    pub async fn get_items_unlabeled(&self, checked: bool) -> Vec<ItemModel> {
        let date_util = DateTime::default();
        let items_model = match ItemEntity::find()
            .filter(items::Column::Labels.is_null().and(items::Column::Checked.eq(checked)))
            .all(&self.db)
            .await
        {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model)
            .filter_map(|model| async move {
                let Ok(item) = Item::from_db(self.db.clone(), &model.id).await else {
                    return None;
                };
                if item.was_archived().await {
                    return None;
                }
                Some(model)
            })
            .collect()
            .await
    }

    pub async fn get_items_no_parent(&self, checked: bool) -> Vec<ItemModel> {
        let date_util = DateTime::default();
        let items_model = match ItemEntity::find()
            .filter(items::Column::ParentId.is_null().and(items::Column::Checked.eq(checked)))
            .all(&self.db)
            .await
        {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model)
            .filter_map(|model| async move {
                let Ok(item) = Item::from_db(self.db.clone(), &model.id).await else {
                    return None;
                };
                if item.was_archived().await {
                    return None;
                }
                Some(model)
            })
            .collect()
            .await
    }

    pub async fn valid_item_by_date(
        &self,
        item_id: &str,
        date: &NaiveDateTime,
        checked: bool,
    ) -> bool {
        let Some(item_model) = self.get_item(item_id).await else {
            return false;
        };
        let Ok(item) = Item::from_db(self.db.clone(), &item_model.id).await else {
            return false;
        };

        // 检查基本条件
        if item_model.checked != checked || item.was_archived().await || !item.has_due() {
            return false;
        }
        let date_util = DateTime::default();
        // 检查截止日期
        item.due()
            .and_then(|d| d.datetime())
            .map(|due| date_util.is_same_day(&due, date))
            .unwrap_or(false)
    }

    pub async fn valid_item_by_date_range(
        &self,
        item_id: &str,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        checked: bool,
    ) -> bool {
        let Some(item_model) = self.get_item(item_id).await else {
            return false;
        };
        let Ok(item) = Item::from_db(self.db.clone(), &item_model.id).await else {
            return false;
        };

        // 检查基本条件
        if item_model.checked != checked || item.was_archived().await || !item.has_due() {
            return false;
        }
        // 检查截止日期
        item.due()
            .and_then(|d| d.datetime())
            .map(|due| due >= start_date && due <= end_date)
            .unwrap_or(false)
    }

    pub async fn valid_item_by_month(
        &self,
        item_id: &str,
        date: &NaiveDateTime,
        checked: bool,
    ) -> bool {
        let Some(item_model) = self.get_item(item_id).await else {
            return false;
        };
        let Ok(item) = Item::from_db(self.db.clone(), &item_model.id).await else {
            return false;
        };

        // 检查基本条件
        if item_model.checked != checked || item.was_archived().await || !item.has_due() {
            return false;
        }
        // 检查截止日期
        item.due()
            .and_then(|d| d.datetime())
            .map(|due| due.year() == date.year() && due.month() == date.month())
            .unwrap_or(false)
    }

    pub async fn get_items_by_overdeue_view(&self, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find()
            .filter(items::Column::Due.is_not_null().and(items::Column::Checked.eq(checked)))
            .all(&self.db)
            .await
        {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model)
            .filter_map(|model| async move {
                let Ok(item) = Item::from_db(self.db.clone(), &model.id).await else {
                    return None;
                };
                if item.was_archived().await {
                    return None;
                };
                let now = Utc::now().naive_utc();
                let date_util = DateTime::default();
                item.due()
                    .and_then(|d| d.datetime())
                    .map(|due| due < now && !DateTime::default().is_same_day(&due, &now))
                    .unwrap_or(false)
                    .then_some(model)
            })
            .collect()
            .await
    }

    pub async fn get_all_items_by_search(&self, search_text: &str) -> Vec<ItemModel> {
        let search_lover = search_text.to_lowercase();
        ItemEntity::find()
            .filter(
                items::Column::Content
                    .contains(&search_lover)
                    .or(items::Column::Description.contains(&search_lover)),
            )
            .all(&self.db)
            .await
            .unwrap()
    }

    // 判断一个项目是否过期了，基于逾期状态和是否被选中
    pub async fn valid_item_by_overdue(&self, item_id: &str, checked: bool) -> bool {
        // 获取项目，失败直接返回false
        let Some(item_model) = self.get_item(item_id).await else {
            return false;
        };
        let Ok(item) = Item::from_db(self.db.clone(), &item_model.id).await else {
            return false;
        };

        // 检查基本条件
        if item_model.checked != checked || item.was_archived().await || !item.has_due() {
            return false;
        }
        let now = Utc::now().naive_utc();
        // 检查截止日期
        item.due()
            .and_then(|d| d.datetime())
            .map(|due| due < now && DateTime::default().is_same_day(&due, &now))
            .unwrap_or(false)
    }

    // labels
    pub async fn labels(&self) -> Vec<LabelModel> {
        LabelEntity::find().all(&self.db).await.unwrap_or_default()
    }

    pub async fn insert_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        let mut active_label: LabelActiveModel = label.into();
        Ok(active_label.insert(&self.db).await?)
    }

    pub async fn update_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        let mut active_label: LabelActiveModel =
            <LabelModel as Into<LabelActiveModel>>::into(label).reset_all();
        Ok(active_label.update(&self.db).await?)
    }

    pub async fn delete_label(&self, id: &str) -> Result<u64, TodoError> {
        Ok(LabelEntity::delete_by_id(id).exec(&self.db).await?.rows_affected)
    }

    pub async fn label_exists(&self, id: &str) -> bool {
        LabelEntity::find_by_id(id).one(&self.db).await.is_ok()
    }

    pub async fn get_label(&self, id: &str) -> Option<LabelModel> {
        LabelEntity::find_by_id(id).one(&self.db).await.unwrap_or_default()
    }

    pub async fn get_labels_by_item_labels(&self, labels: &str) -> Vec<LabelModel> {
        let labels: Vec<String> = labels.split(',').map(|s| s.trim().to_string()).collect();
        LabelEntity::find()
            .filter(labels::Column::Id.is_in(labels))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_label_by_name(&self, name: &str, source_id: &str) -> Option<LabelModel> {
        LabelEntity::find()
            .filter(labels::Column::Name.eq(name).and(labels::Column::SourceId.eq(source_id)))
            .one(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_labels_by_source(&self, source_id: &str) -> Vec<LabelModel> {
        LabelEntity::find()
            .filter(labels::Column::SourceId.eq(source_id))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_all_labels_by_search(&self, search_text: &str) -> Vec<LabelModel> {
        let search_lover = search_text.to_lowercase();
        LabelEntity::find()
            .filter(labels::Column::Name.contains(&search_lover))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    // reminders
    pub async fn reminders(&self) -> Vec<ReminderModel> {
        ReminderEntity::find().all(&self.db).await.unwrap_or_default()
    }

    pub async fn get_reminder(&self, id: &str) -> Option<ReminderModel> {
        ReminderEntity::find_by_id(id).one(&self.db).await.unwrap_or_default()
    }

    pub async fn get_reminders_by_item(&self, item_id: &str) -> Vec<ReminderModel> {
        ReminderEntity::find()
            .filter(reminders::Column::ItemId.eq(item_id))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn insert_reminder(
        &self,
        reminder: ReminderModel,
    ) -> Result<ReminderModel, TodoError> {
        let mut active_reminder: ReminderActiveModel = reminder.into();
        Ok(active_reminder.insert(&self.db).await?)
        // reminder.item.reminder_added (reminder);
    }

    pub async fn delete_reminder(&self, reminder_id: &str) -> Result<u64, TodoError> {
        Ok(ReminderEntity::delete_by_id(reminder_id).exec(&self.db).await?.rows_affected)
        // reminder.item.reminder_deleted (reminder);
    }
}
