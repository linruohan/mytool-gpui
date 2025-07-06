use crate::entity::prelude::*;
use crate::entity::{attachments, items, labels, projects, reminders, sections, AttachmentActiveModel, AttachmentModel, ItemActiveModel, ItemModel, LabelActiveModel, LabelModel, ProjectActiveModel, ProjectModel, ReminderActiveModel, ReminderModel, SectionActiveModel, SectionModel, SourceActiveModel, SourceModel};
use crate::error::TodoError;
use crate::objects::{BaseTrait, Item, Section};
use crate::utils::DateTime;
use chrono::{Datelike, NaiveDateTime, Utc};
use futures::stream::{self, StreamExt};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, Set};

#[derive(Clone, Debug)]
pub struct Store {
    db: DatabaseConnection,
}

impl Store {
    pub async fn new(db: DatabaseConnection) -> Store {
        Self { db }
    }
    // attachments
    pub async fn attachments(&self) -> Vec<AttachmentModel> {
        AttachmentEntity::find().all(&self.db).await.unwrap_or_default()
    }
    pub async fn delete_attachment(&self, id: &str) -> Result<u64, TodoError> {
        Ok(AttachmentEntity::delete_by_id(id)
            .exec(&self.db)
            .await?
            .rows_affected)
        // attachment.item.attachment_deleted (attachment);
    }

    pub async fn insert_attachment(
        &self,
        attachments: AttachmentModel,
    ) -> Result<AttachmentModel, TodoError> {
        let mut active_attachment: AttachmentActiveModel = attachments.into();
        Ok(active_attachment.insert(&self.db).await?)
        // attachment.item.attachment_added (attachment);
    }

    pub async fn get_attachments_by_itemid(
        &self,
        item_id: &str,
    ) -> Vec<AttachmentModel> {
        AttachmentEntity::find()
            .filter(attachments::Column::ItemId.eq(item_id))
            .all(&self.db)
            .await.unwrap_or_default()
    }

    // sources
    pub async fn sources(&self) -> Vec<SourceModel> {
        SourceEntity::find().all(&self.db).await.unwrap_or_default()
    }
    pub async fn get_source(&self, id: &str) -> Option<SourceModel> {
        SourceEntity::find_by_id(id).one(&self.db).await.unwrap_or_default()
    }

    pub async fn insert_source(
        &self,
        sources: SourceModel,
    ) -> Result<SourceModel, TodoError> {
        let mut active_source: SourceActiveModel = sources.into();
        Ok(active_source.insert(&self.db).await?)
    }
    pub async fn delete_source(&self, source_id: &str) -> Result<u64, TodoError> {
        let result = SourceEntity::delete_by_id(source_id)
            .exec(&self.db)
            .await?;
        if result.rows_affected > 0 {
            for project in self.get_projects_by_source(source_id).await {
                self.delete_project(&project.id).await?;
            }
        }
        Ok(1)
    }

    pub async fn update_source(
        &self,
        source: SourceModel,
    ) -> Result<SourceModel, TodoError> {
        let mut active_source: SourceActiveModel = source.into();
        Ok(active_source.update(&self.db).await?)
    }
    // projects
    pub async fn projects(&self) -> Vec<ProjectModel> {
        ProjectEntity::find().all(&self.db).await.unwrap_or_default()
    }
    pub async fn insert_project(
        &self,
        project: ProjectModel,
    ) -> Result<ProjectModel, TodoError> {
        let mut active_project: ProjectActiveModel = project.into();
        Ok(active_project.insert(&self.db).await?)
        //     && let Some(parent) = project.parent()
        // {
        //     parent.add_subproject(project);
        // }
    }
    pub async fn get_project(&self, id: &str) -> Option<ProjectModel> {
        ProjectEntity::find_by_id(id).one(&self.db).await.unwrap_or_default()
    }
    pub async fn get_projects_by_source(
        &self,
        id: &str,
    ) -> Vec<ProjectModel> {
        ProjectEntity::find()
            .filter(projects::Column::SourceId.eq(id))
            .all(&self.db)
            .await.unwrap_or_default()
    }
    pub async fn update_project(
        &self,
        project: ProjectModel,
    ) -> Result<ProjectModel, TodoError> {
        let mut active_project: ProjectActiveModel = project.into();
        Ok(active_project.update(&self.db).await?)
    }
    pub async fn delete_project(&self, id: &str) -> Result<u64, TodoError> {
        let result = ProjectEntity::delete_by_id(id)
            .exec(&self.db)
            .await?;
        if result.rows_affected > 0 {
            for section in self.get_sections_by_project(id).await? {
                self.delete_section(&section.id);
            }
            for item in self.get_items_by_project(id).await? {
                self.delete_item(&item.id).await?;
            }
            for subproject in self.get_subprojects(id).await {
                self.delete_project(&subproject.id).await?;
            }
        }
        Ok(result.rows_affected)
    }
    // pub async fn update_project_id(&self, cur_id: &str, new_id: &str) {
    //     if Database::default().update_project_id(cur_id, new_id) {
    //         if let Some(mut project) = self.get_project(cur_id) {
    //             project.id = Some(new_id.to_string());
    //         }
    //         if Database::default().update_project_section_id(cur_id, new_id) {
    //             for mut section in self.sections() {
    //                 if section.project_id.as_deref() == Some(cur_id) {
    //                     section.project_id = Some(new_id.to_string());
    //                 }
    //             }
    //         }
    //         if Database::default().update_project_item_id(cur_id, new_id) {
    //             for mut item in self.items() {
    //                 if item.project_id.as_deref() == Some(cur_id) {
    //                     item.project_id = Some(new_id.to_string());
    //                 }
    //             }
    //         }
    //     }
    // }
    // pub async fn next_project_child_order(&self, source: &Source) -> i32 {
    //     self.projects()
    //         .iter()
    //         .filter(|i| i.source_id == source.id && !i.is_deleted())
    //         .count() as i32
    // }
    //
    // pub async fn archive_project(&self, project_id: &str) -> Result<u64, TodoError> {
    //     if let Some(mut project) = self.get_project(project_id).await? {
    //         project.is_archived = true;
    //         let items = self.get_items_by_project(project_id).await?;
    //         for item in items {
    //             self.archive_item(&item, true).await;
    //         }
    //     }
    //
    //     if Database::default().archive_project(project.clone()) {
    //         for item in self.get_items_by_project(project) {
    //             self.archive_item(&item, project.is_archived());
    //         }
    //         for section in self.get_sections_by_project(project) {
    //             let mut sec = section.clone();
    //             sec.is_archived = project.is_archived;
    //             self.archive_section(&sec);
    //         }
    //     }
    // }
    //
    pub async fn get_subprojects(&self, id: &str) -> Vec<ProjectModel> {
        ProjectEntity::find()
            .filter(projects::Column::ParentId.eq(id))
            .all(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_inbox_project(&self) -> Vec<ProjectModel> {
        let projects = self.projects().await;

        self.projects()
            .iter()
            .filter(|s| s.is_inbox_project())
            .cloned()
            .collect()
    }
    pub async fn get_all_projects_archived(&self) -> Vec<ProjectModel> {
        ProjectEntity::find().filter(projects::Column::IsArchived.eq(1)).all(&self.db).await.unwrap_or_default()
    }
    pub async fn get_all_projects_by_search(&self, search_text: &str) -> Vec<ProjectModel> {
        let search_lover = search_text.to_lowercase();
        ProjectEntity::find().filter(projects::Column::Name.contains(&search_lover)).all(&self.db).await.unwrap_or_default()
    }

    // // sections
    pub async fn sections(&self) -> Vec<SectionModel> {
        SectionEntity::find().all(&self.db).await.unwrap_or_default()
    }
    pub async fn get_section(&self, id: &str) -> Option<SectionModel> {
        SectionEntity::find_by_id(id).one(&self.db).await.unwrap_or_default()
    }
    pub async fn get_sections_by_project(&self, project_id: &str) -> Vec<SectionModel> {
        SectionEntity::find().filter(sections::Column::ProjectId.eq(project_id)).all(&self.db).await.unwrap_or_default()
    }
    pub async fn get_sections_archived_by_project(&self, project_id: &str) -> Vec<SectionModel> {
        let sections_model = SectionEntity::find().filter(sections::Column::ProjectId.eq(project_id)).all(&self.db).await?;
        stream::iter(sections_model).filter_map(|model| async move {
            if let Ok(section) = Section::from_db(self.db.clone(), &model.id).await {
                if section.was_archived() {
                    return Some(model);
                }
            };
            return None;
        }).collect().await
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
        let mut active_section: SectionActiveModel = section.into();
        Ok(active_section.update(&self.db).await?)
    }
    // pub async fn move_section(&self, section: &Section, project_id: &str) {
    //     if Database::default().move_section(section, project_id)
    //         && Database::default().move_section_items(section)
    //     {
    //         for mut item in section.items() {
    //             item.project_id = Some(project_id.to_string());
    //         }
    //         // section_moved(section, old_project_id);
    //     }
    // }
    // pub async fn update_section_id(&self, cur_id: &str, new_id: &str) {
    //     if Database::default().update_section_id(cur_id, new_id) {
    //         for mut section in self.sections() {
    //             if section.id.as_deref() == Some(cur_id) {
    //                 section.id = Some(new_id.to_string());
    //             }
    //         }
    //         if Database::default().update_section_item_id(cur_id, new_id) {
    //             for mut item in self.items() {
    //                 if item.section_id.as_deref() == Some(cur_id) {
    //                     item.section_id = Some(new_id.to_string());
    //                 }
    //             }
    //         }
    //     }
    // }
    pub async fn archive_section(&self, section_id: &str) -> Result<(), TodoError> {
        let active_model = SectionActiveModel {
            id: sea_orm::Set(section_id.to_string()),
            is_archived: sea_orm::Set(true),
            archived_at: sea_orm::Set(Some(Utc::now().naive_utc())),
            ..SectionEntity::find_by_id(section_id)
                .one(&self.db)
                .await?
                .ok_or(TodoError::NotFound("section not found".to_string()))?
                .into()
        };
        let section_model = active_model.update(&self.db).await?;
        for item in self.get_items_by_section(section_id).await? {
            self.archive_item(&item.id, true).await?;
        }
        let old = section_model.is_archived;
        let mut section_active_model: SectionActiveModel = section_model.into();
        section_active_model.is_archived = Set(!old);
        section_active_model.update(&self.db).await?;
        Ok(())
    }
    pub async fn insert_section(
        &self,
        section: SectionModel,
    ) -> Result<SectionModel, TodoError> {
        let mut active_section: sections::ActiveModel = section.into();
        Ok(active_section.insert(&self.db).await?)
        // section.project.section_added (section);
    }
    pub async fn delete_section(&self, section_id: &str) -> Result<(), TodoError> {
        let result = SectionEntity::delete_by_id(section_id).exec(&self.db).await?;
        if result.rows_affected > 0 {
            let items = ItemEntity::find()
                .filter(items::Column::SectionId.eq(section_id))
                .all(&self.db)
                .await?;
            for item in items {
                self.delete_item(&*item.id).await?;
            }
        }
        Ok(())
    }

    // // items
    pub async fn items(&self) -> Vec<ItemModel> {
        ItemEntity::find().all(&self.db).await.unwrap_or_default()
    }

    pub async fn insert_item(&self, item: ItemModel, insert: bool) -> Result<(), TodoError> {
        let mut active_model: ItemActiveModel = item.into();
        let item_model = active_model.insert(&self.db).await?;
        self.add_item(item_model.clone(), insert);
        Ok(())
    }

    pub async fn add_item(&self, item: ItemModel, insert: bool) {
        if (insert) {
            // if let Some(parent) = item.parent() {
            //     parent.item_added(item);
            // } else if let Some(section) = item.section() {
            //     section.item_added(item);
            // } else if let Some(project) = item.project() {
            //     project.item_added(item);
            // }
        }
        // Services.EventBus.get_default ().update_items_position (item.project_id, item.section_id);
    }

    // pub async fn update_item(&self, item: &Item, update_id: &str) {
    //     if Database::default().update_item(item) {
    //         // self.item_updated(item.clone(), update_id.clone());
    //     }
    // }
    // pub async fn update_item_pin(&self, item: &Item) {
    //     if Database::default().update_item(item) {
    //         item.pin_updated();
    //     }
    // }
    // pub async fn move_item(&self, item: &Item, project_id: &str, section_id: &str) {
    //     if Database::default().move_item(item) {
    //         for subitem in self.get_subitems(item) {
    //             let mut sub = subitem.clone();
    //             sub.project_id = item.project_id.clone();
    //             self.move_item(&sub, "", "");
    //         }
    //         if let Some(section_id) = item.section_id.clone()
    //             && let Some(section) = self.get_section(&section_id)
    //         {
    //             section.update_count();
    //         }
    //         if let Some(project_id) = item.project_id.clone()
    //             && let Some(project) = self.get_project(&project_id)
    //         {
    //             project.update_count();
    //         }
    //     }
    // }
    //
    pub async fn delete_item(&self, item_id: &str) -> Result<(), TodoError> {
        Box::pin(async move {
            let result = ItemEntity::delete_by_id(item_id).exec(&self.db).await?;
            let mut subitems = ItemEntity::find().filter(items::Column::ParentId.eq(item_id)).all(&self.db).await?;
            for item in subitems {
                self.delete_item(&item.id).await?
            };
            Ok(())
            // if let Some(p) = item.project() {
            //     p.item_deleted(item)
            // }
            // if item.has_section()
            //     && let Some(s) = item.section()
            // {
            //     s.item_deleted(item)
            // }
        }).await
    }
    pub async fn archive_item(&self, item_id: &str, archived: bool) -> Result<(), TodoError> {
        Box::pin(async move {
            let item = Item::from_db(self.db.clone(), item_id).await?;
            if archived {
                item.archived();
            } else {
                item.unarchived();
            };
            let mut subitems = ItemEntity::find().filter(items::Column::ParentId.eq(item_id)).all(&self.db).await?;
            for item in subitems {
                self.archive_item(&item.id, archived).await?
            };
            Ok(())
        }
        ).await
    }

    pub async fn complete_item(&self, item_id: &str, checked: bool, complete_subitems: bool) -> Result<(), TodoError> {
        Box::pin(async move {
            let active_model = ItemActiveModel {
                id: sea_orm::Set(item_id.to_string()),
                checked: sea_orm::Set(checked),
                completed_at: sea_orm::Set(Some(Utc::now().naive_utc())),
                ..ItemEntity::find_by_id(item_id)
                    .one(&self.db)
                    .await?
                    .ok_or(TodoError::NotFound("item not found".to_string()))?
                    .into()
            };
            let item_model = active_model.update(&self.db).await?;
            if complete_subitems {
                let mut subitems = ItemEntity::find().filter(items::Column::ParentId.eq(item_id)).all(&self.db).await?;
                for item in subitems {
                    self.complete_item(&item.id, item_model.checked, complete_subitems).await?
                }
            };
            if let Some(parent) = ItemEntity::find().filter(items::Column::ParentId.eq(item_id)).one(&self.db).await? {
                self.complete_item(&parent.id, item_model.checked, false).await?
            };
            Ok(())
        }).await
    }
    pub async fn update_item_id(&self, item_id: &str, new_id: &str) -> Result<(), TodoError> {
        let item_model = ItemActiveModel {
            id: sea_orm::Set(new_id.to_string()),
            ..ItemEntity::find_by_id(item_id).one(&self.db).await?.ok_or(TodoError::NotFound("item not found".to_string()))?.into()
        };
        item_model.update(&self.db).await?;

        ItemEntity::update_many().set(ItemActiveModel {
            parent_id: sea_orm::Set(Some(new_id.to_string())),
            ..Default::default()
        }).filter(items::Column::ParentId.eq(item_id)).exec(&self.db).await?;
        Ok(())
    }
    pub async fn next_item_child_order(&self, project_id: &str, section_id: &str) -> i32 {
        ItemEntity::find().filter(items::Column::ProjectId.eq(project_id).and(
            items::Column::SectionId.eq(section_id)
        )).count(&self.db).await.unwrap_or(0) as i32
    }
    pub async fn get_item(&self, id: &str) -> Result<Option<ItemModel>, TodoError> {
        Ok(ItemEntity::find_by_id(id).one(&self.db).await?)
    }

    pub async fn get_items_by_section(&self, section_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        Ok(ItemEntity::find().filter(
            items::Column::SectionId.eq(section_id)
        ).all(&self.db).await?)
    }

    pub async fn get_subitems(&self, item_id: &str) -> Vec<ItemModel> {
        ItemEntity::find().filter(items::Column::ParentId.eq(item_id)).all(&self.db).await.unwrap_or_default()
    }
    pub async fn get_items_completed(&self) -> Vec<ItemModel> {
        ItemEntity::find().filter(
            items::Column::Checked.eq(1).and(items::Column::SectionId.eq(""))
        ).all(&self.db).await.unwrap_or_default()
        // .filter(|s| s.checked == Some(1) && !s.was_archived())
    }
    pub async fn get_item_by_ics(&self, ics: &str) -> Option<ItemModel> {
        ItemEntity::find()
            .filter(items::Column::Id.eq(ics))
            .one(&self.db)
            .await
            .unwrap_or_default()
    }

    pub async fn get_items_has_labels(&self) -> Vec<ItemModel> {
        ItemEntity::find()
            .filter(items::Column::Labels.is_not_null())
            .all(&self.db)
            .await
            .unwrap_or_default()
        // .filter(|s| s.has_labels() && s.completed() && !s.was_archived())
    }

    pub async fn get_items_by_label(&self, label_id: &str, checked: bool) -> Vec<ItemModel> {
        ItemEntity::find().filter(
            items::Column::Labels.is_not_null().and(
                items::Column::Checked.eq(1)
            )).all(&self.db).await.unwrap_or_default()
        // .filter(|i| i.has_label(label_id) && i.checked() == checked && !i.was_archived())
    }

    pub async fn get_items_checked(&self) -> Result<Vec<ItemModel>, TodoError> {
        Ok(ItemEntity::find().filter(
            items::Column::Checked.eq(1)
        ).all(&self.db).await?)
    }
    pub async fn get_items_checked_by_project(&self, project_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        Ok(ItemEntity::find().filter(
            items::Column::ProjectId.eq(project_id).and(
                items::Column::Checked.eq(1)
            )).all(&self.db).await?)
    }
    pub async fn get_subitems_uncomplete(&self, item_id: &str) -> Vec<ItemModel> {
        ItemEntity::find().filter(items::Column::ParentId.eq(item_id).and(
            items::Column::Checked.eq(0)
        )).all(&self.db).await.unwrap_or_default()
    }
    pub async fn get_items_by_project(&self, project_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        Ok(ItemEntity::find().filter(items::Column::ProjectId.eq(project_id)).all(&self.db).await?)
    }
    pub async fn get_items_by_project_pinned(&self, project_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        Ok(ItemEntity::find().filter(items::Column::ProjectId.eq(project_id).and(
            items::Column::Pinned.eq(1)
        )).all(&self.db).await?)
    }
    pub async fn get_items_by_date(&self, date: &NaiveDateTime, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find().filter(items::Column::Pinned.eq(1).and(
            items::Column::Checked.eq(checked)
        )).all(&self.db).await {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model).filter_map(|model| async move {
            self.valid_item_by_date(&*model.id, date, checked).await
                .then_some(model)
        }).collect().await
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
            .filter(items::Column::Due.is_not_null().and(items::Column::Checked.eq(checked))
            ).all(&self.db).await.unwrap_or_default()
        //     i.has_due() && i.due().is_recurring && i.checked() == checked && !i.was_archived()
    }
    pub async fn get_items_by_date_range(
        &self,
        start_date: NaiveDateTime,
        end_date: NaiveDateTime,
        checked: bool,
    ) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find().filter(items::Column::Pinned.eq(1).and(
            items::Column::Checked.eq(checked)
        )).all(&self.db).await {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model).filter_map(|model| async move {
            self.valid_item_by_date_range(&*model.id, start_date, end_date, checked).await
                .then_some(model)
        }).collect().await
    }
    pub async fn get_items_by_month(&self, date: &NaiveDateTime, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find().filter(items::Column::Pinned.eq(1).and(
            items::Column::Checked.eq(checked)
        )).all(&self.db).await {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model).filter_map(|model| async move {
            self.valid_item_by_month(&*model.id, date, checked).await
                .then_some(model)
        }).collect().await
    }
    pub async fn get_items_pinned(&self, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find().filter(items::Column::Pinned.eq(1).and(
            items::Column::Checked.eq(checked)
        )).all(&self.db).await {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model).filter_map(|model| async move {
            let Ok(item) = Item::from_db(self.db.clone(), &model.id).await else { return None };
            if item.was_archived() {
                return None;
            }
            Some(model)
        }).collect().await
    }
    pub async fn get_items_by_priority(&self, priority: i32, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find().filter(items::Column::Priority.eq(priority).and(
            items::Column::Checked.eq(checked)
        )).all(&self.db).await {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model).filter_map(|model| async move {
            let Ok(item) = Item::from_db(self.db.clone(), &model.id).await else { return None };
            if item.was_archived() {
                return None;
            }
            Some(model)
        }).collect().await
    }

    pub async fn get_items_by_scheduled(&self, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find().filter(items::Column::Due.is_not_null().and(
            items::Column::Checked.eq(checked)
        )).all(&self.db).await {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model).filter_map(|model| async move {
            let Ok(item) = Item::from_db(self.db.clone(), &model.id).await else { return None };
            if item.was_archived() {
                return None;
            }
            let now = Utc::now().naive_utc();
            // 检查截止日期
            item.due()
                .and_then(|d| d.datetime())
                .map(|due| due > now)
                .unwrap_or(false).then_some(model)
        }).collect().await
    }

    pub async fn get_items_unlabeled(&self, checked: bool) -> Vec<ItemModel> {
        let date_util = DateTime::default();
        let items_model = match ItemEntity::find().filter(items::Column::Labels.is_null().and(
            items::Column::Checked.eq(checked)
        )).all(&self.db).await {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model).filter_map(|model| async move {
            let Ok(item) = Item::from_db(self.db.clone(), &model.id).await else { return None };
            if item.was_archived() {
                return None;
            }
            Some(model)
        }).collect().await
    }
    pub async fn get_items_no_parent(&self, checked: bool) -> Vec<ItemModel> {
        let date_util = DateTime::default();
        let items_model = match ItemEntity::find().filter(items::Column::ParentId.is_null().and(
            items::Column::Checked.eq(checked)
        )).all(&self.db).await {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model).filter_map(|model| async move {
            let Ok(item) = Item::from_db(self.db.clone(), &model.id).await else { return None };
            if item.was_archived() {
                return None;
            }
            Some(model)
        }).collect().await
    }
    pub async fn valid_item_by_date(
        &self,
        item_id: &str,
        date: &NaiveDateTime,
        checked: bool,
    ) -> bool {
        let Ok(Some(item_model)) = self.get_item(item_id).await else { return false };
        let Ok(item) = Item::from_db(self.db.clone(), &item_model.id).await else { return false };

        // 检查基本条件
        if item.checked() != checked || item.was_archived() || !item.has_due() {
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
        let Ok(Some(item_model)) = self.get_item(item_id).await else { return false };
        let Ok(item) = Item::from_db(self.db.clone(), &item_model.id).await else { return false };

        // 检查基本条件
        if item.checked() != checked || item.was_archived() || !item.has_due() {
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
        let Ok(Some(item_model)) = self.get_item(item_id).await else { return false };
        let Ok(item) = Item::from_db(self.db.clone(), &item_model.id).await else { return false };

        // 检查基本条件
        if item.checked() != checked || item.was_archived() || !item.has_due() {
            return false;
        }
        // 检查截止日期
        item.due()
            .and_then(|d| d.datetime())
            .map(|due| due.year() == date.year() && due.month() == date.month())
            .unwrap_or(false)
    }

    pub async fn get_items_by_overdeue_view(&self, checked: bool) -> Vec<ItemModel> {
        let items_model = match ItemEntity::find().filter(items::Column::Due.is_not_null().and(
            items::Column::Checked.eq(checked)
        )).all(&self.db).await {
            Ok(items) => items,
            Err(_) => return vec![],
        };
        stream::iter(items_model).filter_map(|model| async move {
            let Ok(item) = Item::from_db(self.db.clone(), &model.id).await else { return None };
            if item.was_archived() {
                return None;
            };
            let now = Utc::now().naive_utc();
            let date_util = DateTime::default();
            item.due()
                .and_then(|d| d.datetime())
                .map(|due| due < now && !DateTime::default().is_same_day(&due, &now))
                .unwrap_or(false).then_some(model)
        }).collect().await
    }

    pub async fn get_all_items_by_search(
        &self,
        search_text: &str,
    ) -> Vec<ItemModel> {
        let search_lover = search_text.to_lowercase();
        ItemEntity::find()
            .filter(
                items::Column::Content
                    .contains(&search_lover)
                    .or(items::Column::Description.contains(&search_lover)),
            )
            .all(&self.db)
            .await.unwrap()
    }
    // 判断一个项目是否过期了，基于逾期状态和是否被选中
    pub async fn valid_item_by_overdue(&self, item_id: &str, checked: bool) -> bool {
        // 获取项目，失败直接返回false
        let Ok(Some(item_model)) = self.get_item(item_id).await else { return false };
        let Ok(item) = Item::from_db(self.db.clone(), &item_model.id).await else { return false };

        // 检查基本条件
        if item.checked() != checked || item.was_archived() || !item.has_due() {
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
        let mut active_label: LabelActiveModel = label.into();
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
    pub async fn get_labels_by_item_labels(
        &self,
        labels: &str,
    ) -> Vec<LabelModel> {
        let labels: Vec<String> = labels.split(',').map(|s| s.trim().to_string()).collect();
        LabelEntity::find()
            .filter(labels::Column::Id.is_in(labels))
            .all(&self.db)
            .await.unwrap_or_default()
    }
    pub async fn get_label_by_name(
        &self,
        name: &str,
        source_id: &str,
    ) -> Option<LabelModel> {
        LabelEntity::find()
            .filter(
                labels::Column::Name
                    .eq(name)
                    .and(labels::Column::SourceId.eq(source_id)),
            )
            .one(&self.db)
            .await.unwrap_or_default()
    }
    pub async fn get_labels_by_source(
        &self,
        source_id: &str,
    ) -> Vec<LabelModel> {
        LabelEntity::find()
            .filter(labels::Column::SourceId.eq(source_id))
            .all(&self.db)
            .await.unwrap_or_default()
    }
    pub async fn get_all_labels_by_search(
        &self,
        search_text: &str,
    ) -> Vec<LabelModel> {
        let search_lover = search_text.to_lowercase();
        LabelEntity::find()
            .filter(labels::Column::Name.contains(&search_lover))
            .all(&self.db)
            .await.unwrap_or_default()
    }
    // reminders
    pub async fn reminders(&self) -> Vec<ReminderModel> {
        ReminderEntity::find().all(&self.db).await.unwrap_or_default()
    }
    pub async fn get_reminder(&self, id: &str) -> Option<ReminderModel> {
        ReminderEntity::find_by_id(id).one(&self.db).await.unwrap_or_default()
    }

    pub async fn get_reminders_by_item(
        &self,
        item_id: &str,
    ) -> Vec<ReminderModel> {
        ReminderEntity::find()
            .filter(reminders::Column::ItemId.eq(item_id))
            .all(&self.db)
            .await.unwrap_or_default()
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
