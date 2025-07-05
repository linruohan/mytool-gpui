use crate::entity::{
    attachments, items, labels,
    prelude::{Attachments, Items, Labels, Projects, Reminders, Sections, Sources},
    projects, reminders, sections, sources,
};
use crate::error::TodoError;
use crate::objects::Item;
use crate::utils::DateTime;
use chrono::{Datelike, Local, NaiveDateTime};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DeleteResult, EntityTrait, QueryFilter,
};

#[derive(Clone, Debug)]
pub struct Store {
    db: DatabaseConnection,
}

impl Store {
    pub async fn new(db: DatabaseConnection) -> Store {
        Self { db }
    }
    // attachments
    pub async fn attachments(&self) -> Result<Vec<attachments::Model>, TodoError> {
        Ok(Attachments::find().all(&self.db).await?)
    }
    pub async fn delete_attachment(&self, id: &str) -> Result<u64, TodoError> {
        Ok(Attachments::delete_by_id(id)
            .exec(&self.db)
            .await?
            .rows_affected)

        // attachment.deleted ();
        // attachment_deleted (attachment);
        // _attachments.remove (attachment);
        //
        // attachment.item.attachment_deleted (attachment);
    }

    pub async fn insert_attachment(
        &self,
        attachments: attachments::Model,
    ) -> Result<attachments::Model, TodoError> {
        let mut active_attachment: attachments::ActiveModel = attachments.into();
        Ok(active_attachment.insert(&self.db).await?)
        // attachment.item.attachment_added (attachment);
    }

    pub async fn get_attachments_by_itemid(
        &self,
        item_id: &str,
    ) -> Result<Vec<attachments::Model>, TodoError> {
        Ok(Attachments::find()
            .filter(attachments::Column::ItemId.eq(item_id))
            .all(&self.db)
            .await?)
    }

    // sources
    pub async fn sources(&self) -> Result<Vec<sources::Model>, TodoError> {
        Ok(Sources::find().all(&self.db).await?)
    }
    pub async fn get_source(&self, id: &str) -> Result<Option<sources::Model>, TodoError> {
        Ok(Sources::find_by_id(id).one(&self.db).await?)
    }

    pub async fn insert_source(
        &self,
        sources: sources::Model,
    ) -> Result<sources::Model, TodoError> {
        let mut active_source: sources::ActiveModel = sources.into();
        Ok(active_source.insert(&self.db).await?)
    }
    pub async fn delete_source(&self, source_id: &str) -> Result<u64, TodoError> {
        Ok(Sources::delete_by_id(source_id)
            .exec(&self.db)
            .await?
            .rows_affected)
        // for project in self.get_projects_by_source(source.id()) {
        //     self.delete_project(&project);
        // }
    }

    pub async fn update_source(
        &self,
        source: &sources::Model,
    ) -> Result<sources::Model, TodoError> {
        let mut active_source: sources::ActiveModel = source.into();
        Ok(active_source.update(&self.db).await?)
    }
    // projects
    pub async fn projects(&self) -> Result<Vec<projects::Model>, TodoError> {
        Ok(Projects::find().all(&self.db).await?)
    }
    pub async fn insert_project(
        &self,
        project: projects::Model,
    ) -> Result<projects::Model, TodoError> {
        let mut active_project: projects::ActiveModel = project.into();
        Ok(active_project.insert(&self.db).await?)
        //     && let Some(parent) = project.parent()
        // {
        //     parent.add_subproject(project);
        // }
    }
    pub async fn get_project(&self, id: &str) -> Result<Option<projects::Model>, TodoError> {
        Ok(Projects::find_by_id(id).one(&self.db).await?)
    }
    pub async fn get_projects_by_source(
        &self,
        id: &str,
    ) -> Result<Vec<projects::Model>, TodoError> {
        Ok(Projects::find()
            .filter(projects::Column::SourceId.eq(id))
            .all(&self.db)
            .await?)
    }
    pub async fn update_project(
        &self,
        project: projects::Model,
    ) -> Result<projects::Model, TodoError> {
        let mut active_project: projects::ActiveModel = project.into();
        Ok(active_project.update(&self.db).await?)
    }
    pub async fn delete_project(&self, id: &str) -> Result<u64, TodoError> {
        Ok(Projects::delete_by_id(id)
            .exec(&self.db)
            .await?
            .rows_affected)
        // for section in self.get_sections_by_project(id) {
        //         self.delete_section(&section);
        //     }
        //     for item in self.get_items_by_project(project) {
        //         self.delete_item(&item);
        //     }
        //     for subproject in self.get_subprojects(project_id) {
        //         self.delete_project(&subproject);
        //     }
    }
    pub async fn update_project_id(&self, cur_id: &str, new_id: &str) {
        if Database::default().update_project_id(cur_id, new_id) {
            if let Some(mut project) = self.get_project(cur_id) {
                project.id = Some(new_id.to_string());
            }
            if Database::default().update_project_section_id(cur_id, new_id) {
                for mut section in self.sections() {
                    if section.project_id.as_deref() == Some(cur_id) {
                        section.project_id = Some(new_id.to_string());
                    }
                }
            }
            if Database::default().update_project_item_id(cur_id, new_id) {
                for mut item in self.items() {
                    if item.project_id.as_deref() == Some(cur_id) {
                        item.project_id = Some(new_id.to_string());
                    }
                }
            }
        }
    }
    pub async fn next_project_child_order(&self, source: &Source) -> i32 {
        self.projects()
            .iter()
            .filter(|i| i.source_id == source.id && !i.is_deleted())
            .count() as i32
    }

    pub async fn archive_project(&self, project_id: &str) -> Result<u64, TodoError> {
        if let Some(mut project) = self.get_project(project_id).await? {
            project.is_archived = true;
            let items = self.get_items_by_project(project_id).await?;
            for item in items {
                self.archive_item(&item, true).await;
            }
        }

        if Database::default().archive_project(project.clone()) {
            for item in self.get_items_by_project(project) {
                self.archive_item(&item, project.is_archived());
            }
            for section in self.get_sections_by_project(project) {
                let mut sec = section.clone();
                sec.is_archived = project.is_archived;
                self.archive_section(&sec);
            }
        }
    }

    pub async fn get_subprojects(&self, id: &str) -> Vec<Project> {
        self.projects()
            .iter()
            .filter(|s| s.parent_id.as_deref() == Some(id))
            .cloned()
            .collect()
    }

    pub async fn get_inbox_project(&self) -> Vec<Project> {
        self.projects()
            .iter()
            .filter(|s| s.is_inbox_project())
            .cloned()
            .collect()
    }
    pub async fn get_all_projects_archived(&self) -> Vec<Project> {
        self.projects()
            .iter()
            .filter(|s| s.is_archived())
            .cloned()
            .collect()
    }
    pub async fn get_all_projects_by_search(&self, search_text: &str) -> Vec<Project> {
        let search_lover = search_text.to_lowercase();
        self.projects()
            .iter()
            .filter(|s| s.name.contains(&search_lover) && !s.is_archived())
            .cloned()
            .collect()
    }

    // sections
    pub async fn sections(&self) -> Result<Vec<sections::Model>, TodoError> {
        Ok(Sections::find().all(&self.db).await?)
    }
    pub async fn get_section(&self, id: &str) -> Option<Section> {
        self.sections()
            .iter()
            .find(|s| s.id.as_deref() == Some(id))
            .cloned()
    }
    pub async fn get_sections_by_project(&self, project_id: &str) -> Vec<Section> {
        self.sections()
            .iter()
            .filter(|s| s.project_id == project.id)
            .cloned()
            .collect()
    }
    pub async fn get_sections_archived_by_project(&self, project: &Project) -> Vec<Section> {
        self.sections()
            .iter()
            .filter(|s| s.project_id == project.id && s.was_archived())
            .cloned()
            .collect()
    }
    pub async fn get_all_sections_by_search(&self, search_text: &str) -> Vec<Section> {
        let search_lover = search_text.to_lowercase();
        self.sections()
            .iter()
            .filter(|s| {
                s.name
                    .as_deref()
                    .map(|name| name.contains(&search_lover))
                    .unwrap_or(false)
                    && !s.was_archived()
            })
            .cloned()
            .collect()
    }
    pub async fn update_section(&self, section: &Section) {
        if Database::default().update_section(section) {
            // section.updated ();
            todo!()
        }
    }
    pub async fn move_section(&self, section: &Section, project_id: &str) {
        if Database::default().move_section(section, project_id)
            && Database::default().move_section_items(section)
        {
            for mut item in section.items() {
                item.project_id = Some(project_id.to_string());
            }
            // section_moved(section, old_project_id);
        }
    }
    pub async fn update_section_id(&self, cur_id: &str, new_id: &str) {
        if Database::default().update_section_id(cur_id, new_id) {
            for mut section in self.sections() {
                if section.id.as_deref() == Some(cur_id) {
                    section.id = Some(new_id.to_string());
                }
            }
            if Database::default().update_section_item_id(cur_id, new_id) {
                for mut item in self.items() {
                    if item.section_id.as_deref() == Some(cur_id) {
                        item.section_id = Some(new_id.to_string());
                    }
                }
            }
        }
    }
    pub async fn archive_section(&self, section: &Section) {
        if Database::default().archive_section(section) {
            for item in self.get_items_by_section(section.id()) {
                self.archive_item(&item, section.is_archived());
            }
            if section.is_archived() {
                section.archived();
                // section_archived(section);
            } else {
                section.unarchived();
                // section_unarchived (section);
            }
        }
    }
    pub async fn insert_section(
        &self,
        section: sections::Model,
    ) -> Result<sections::Model, TodoError> {
        let mut active_section: sections::ActiveModel = section.into();
        Ok(active_section.insert(&self.db).await)

        // section.project.section_added (section);
    }
    pub async fn delete_section(&self, section: &Section) {
        if Database::default().delete_section(section) {
            for item in section.items() {
                self.delete_item(&item);
            }
            // section.deleted ();
            // section_deleted (section);
            // _sections.remove (section);
        }
    }

    // items
    pub async fn items(&self) -> Result<Vec<items::Model>, TodoError> {
        Ok(Items::find().all(&self.db).await?)
    }

    pub async fn insert_item(&self, item: &Item, insert: bool) {
        if Database::default().insert_item(item) {
            self.add_item(item, insert);
        }
    }

    pub async fn add_item(&self, item: &Item, insert: bool) {
        let mut item1 = item.clone();
        // self.items().push(item);
        // item_added (item, insert);
        if (insert) {
            if let Some(parent) = item.parent() {
                parent.item_added(item);
            } else if let Some(section) = item.section() {
                section.item_added(item);
            } else if let Some(project) = item.project() {
                project.item_added(item);
            }
        }
        // Services.EventBus.get_default ().update_items_position (item.project_id, item.section_id);
    }

    pub async fn update_item(&self, item: &Item, update_id: &str) {
        if Database::default().update_item(item) {
            // self.item_updated(item.clone(), update_id.clone());
        }
    }
    pub async fn update_item_pin(&self, item: &Item) {
        if Database::default().update_item(item) {
            item.pin_updated();
        }
    }
    pub async fn move_item(&self, item: &Item, project_id: &str, section_id: &str) {
        if Database::default().move_item(item) {
            for subitem in self.get_subitems(item) {
                let mut sub = subitem.clone();
                sub.project_id = item.project_id.clone();
                self.move_item(&sub, "", "");
            }
            if let Some(section_id) = item.section_id.clone()
                && let Some(section) = self.get_section(&section_id)
            {
                section.update_count();
            }
            if let Some(project_id) = item.project_id.clone()
                && let Some(project) = self.get_project(&project_id)
            {
                project.update_count();
            }
        }
    }

    pub async fn delete_item(&self, item: &Item) {
        if Database::default().delete_item(item) {
            for subitem in self.get_subitems(item) {
                self.delete_item(&subitem);
            }
            if let Some(p) = item.project() {
                p.item_deleted(item)
            }
            if item.has_section()
                && let Some(s) = item.section()
            {
                s.item_deleted(item)
            }
        }
    }
    pub async fn archive_item(&self, item: &Item, archived: bool) {
        if archived {
            item.archived();
        } else {
            item.unarchived();
        }
        for subitem in self.get_subitems(item) {
            self.archive_item(&subitem, archived);
        }
    }
    pub async fn item_updated(&self, item: &Item, update_id: &str) {
        todo!()
    }
    pub async fn complete_item(&self, item: &Item) {
        if Database::default().complete_item(item) {
            for mut subitem in self.get_subitems(item) {
                subitem.checked = item.checked;
                subitem.completed_at = item.completed_at.clone();
                self.complete_item(&subitem);
            }
            item.update("");
            self.item_updated(item, "");
            todo!();
            // Services.EventBus.get_default ().checked_toggled (item, old_checked);
            if let Some(mut parent) = item.parent().filter(|_| !item.checked()) {
                parent.checked = item.checked;
                parent.completed_at = item.completed_at.clone();
                self.complete_item(&parent);
            }
        }
    }
    pub async fn update_item_id(&self, cur_id: &str, new_id: &str) {
        if Database::default().update_item_id(cur_id, new_id) {
            for mut item in self.items() {
                if item.id.as_deref() == Some(cur_id) {
                    item.id = Some(new_id.to_string());
                }
            }
            if Database::default().update_item_child_id(cur_id, new_id) {
                for mut item in self.items() {
                    if item.parent_id.as_deref() == Some(cur_id) {
                        item.parent_id = Some(new_id.to_string());
                    }
                }
            }
        }
    }
    pub async fn next_item_child_order(&self, project_id: &str, section_id: &str) -> i32 {
        // self.items()
        // .iter()
        // .filter(|i|
        //     i.project_id.as_deref() == Some(project_id) &&
        //     i.section_id.as_deref() == Some(section_id)
        // )
        // .count() as i32
        self.items().iter().fold(0, |sub, i| {
            if i.project_id.as_deref() == Some(project_id)
                && i.section_id.as_deref() == Some(section_id)
            {
                sub + 1
            } else {
                sub
            }
        })
    }
    pub async fn get_item(&self, id: &str) -> Result<Option<items::Model>, TodoError> {
        Ok(Items::find_by_id(id).one(&self.db).await?)
    }

    pub async fn get_items_by_section(&self, section_id: &str) -> Result<Vec<items::Model>, TodoError> {
        Ok(Items::find().filter(
            items::Column::SectionId.eq(section_id)
        ).all(&self.db).await?)
    }

    pub async fn get_subitems(&self, item_id: &str) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|s| s.parent_id.as_deref() == Some(item_id))
            .cloned()
            .collect()
    }

    pub async fn get_items_completed(&self) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|s| s.checked == Some(1) && !s.was_archived())
            .cloned()
            .collect()
    }
    pub async fn get_item_by_ics(&self, ics: &str) -> Option<Item> {
        self.items()
            .iter()
            .find(|i| i.id.as_deref() == Some(ics))
            .cloned()
    }
    pub async fn get_items_has_labels(&self) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|s| s.has_labels() && s.completed() && !s.was_archived())
            .cloned()
            .collect()
    }

    pub async fn get_items_by_label(&self, label_id: &str, checked: bool)  -> Result<Vec<items::Model>, TodoError> {
        Ok(Items::find().filter(
            items::Column::.eq(project_id).and(
                items::Column::Checked.eq(1)
            )).all(&self.db).await?)
        self.items()
            .iter()
            .filter(|i| i.has_label(label_id) && i.checked() == checked && !i.was_archived())
            .cloned()
            .collect()
    }

    pub async fn get_items_checked(&self) -> Result<Vec<items::Model>, TodoError> {
        Ok(Items::find().filter(
            items::Column::Checked.eq(1)
        ).all(&self.db).await?)
    }
    pub async fn get_items_checked_by_project(&self, project_id: &str) -> Result<Vec<items::Model>, TodoError> {
        Ok(Items::find().filter(
            items::Column::ProjectId.eq(project_id).and(
                items::Column::Checked.eq(1)
            )).all(&self.db).await?)
    }
    pub async fn get_subitems_uncomplete(&self, item: &Item) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| i.parent_id == i.id && !i.checked())
            .cloned()
            .collect()
    }
    pub async fn get_items_by_project(&self, project_id: &str) -> Result<Vec<items::Model>, TodoError> {
        Ok(Items::find().filter(items::Column::ProjectId.eq(project_id)).all(&self.db).await?)
    }
    pub async fn get_items_by_project_pinned(&self, project_id: &str) -> Result<Vec<items::Model>, TodoError> {
        Ok(Items::find().filter(items::Column::ProjectId.eq(project_id).and(
            items::Column::Pinned.eq(1)
        )).all(&self.db).await?)
    }
    pub async fn get_items_by_date(&self, date: &NaiveDateTime, checked: bool) -> Result<Vec<items::Model>, TodoError> {
        Ok(Items::find())
            .filter(
                items::Column::Due
                    .is_not_null()
                    .and(items::Column::Checked.eq(checked))
                    .and(items::Column::AddedAt.eq(date)),
            )
            .all(&self.db)
            .await?)
        self.items()
            .iter()
            .filter(|i| self.valid_item_by_date(i, date, checked))
            .cloned()
            .collect()
    }
    pub async fn get_items_no_date(&self, checked: bool) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| !i.has_due() && i.checked() == checked)
            .cloned()
            .collect()
    }
    pub async fn get_items_repeating(&self, checked: bool) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| {
                i.has_due() && i.due().is_recurring && i.checked() == checked && !i.was_archived()
            })
            .cloned()
            .collect()
    }
    pub async fn get_items_by_date_range(
        &self,
        start_date: &NaiveDateTime,
        end_date: &NaiveDateTime,
        checked: bool,
    ) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|s| self.valid_item_by_date_range(s, start_date, end_date, checked))
            .cloned()
            .collect()
    }
    pub async fn get_items_by_month(&self, date: &NaiveDateTime, checked: bool) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|s| self.valid_item_by_month(s, date, checked))
            .cloned()
            .collect()
    }
    pub async fn get_items_pinned(&self, checked: bool) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| i.pinned == Some(1) && i.checked() && !i.was_archived())
            .cloned()
            .collect()
    }
    pub async fn get_items_by_priority(&self, priority: i32, checked: bool) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| i.priority == Some(priority) && i.checked() && !i.was_archived())
            .cloned()
            .collect()
    }
    pub async fn get_items_with_reminders(&self) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| i.has_reminders() && i.completed() && !i.was_archived())
            .cloned()
            .collect()
    }
    pub async fn get_items_by_scheduled(&self, checked: bool) -> Vec<Item> {
        let now = Local::now().naive_local();
        self.items()
            .iter()
            .filter(|i| {
                i.has_due()
                    && !i.was_archived()
                    && i.checked()
                    && i.due().datetime().filter(|d| d > &now).is_some()
            })
            .cloned()
            .collect()
    }

    pub async fn get_items_unlabeled(&self, checked: bool) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|s| s.labels().is_empty() && s.checked() == checked && !s.was_archived())
            .cloned()
            .collect()
    }
    pub async fn get_items_no_parent(&self, checked: bool) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| !i.was_archived() && i.checked() == checked && !i.has_parent())
            .cloned()
            .collect()
    }
    pub async fn valid_item_by_date(
        &self,
        item: &Item,
        date: &NaiveDateTime,
        checked: bool,
    ) -> bool {
        if item.has_due() || item.was_archived() {
            return false;
        }
        item.checked() == checked
            && item
            .due()
            .datetime()
            .is_some_and(|dt| DateTime::default().is_same_day(&dt, date))
    }

    pub async fn valid_item_by_date_range(
        &self,
        item: &Item,
        start_date: &NaiveDateTime,
        end_date: &NaiveDateTime,
        checked: bool,
    ) -> bool {
        let date_util = DateTime::default();

        !(item.has_due() || item.was_archived())
            && item.checked() == checked
            && item.due().datetime().is_some_and(|dt| {
            let date = date_util.get_date_only(&dt);
            let start = date_util.get_date_only(start_date);
            let end = date_util.get_date_only(end_date);
            date >= start && date <= end
        })
    }
    pub async fn valid_item_by_month(
        &self,
        item: &Item,
        date: &NaiveDateTime,
        checked: bool,
    ) -> bool {
        !(item.has_due() || item.was_archived())
            && item.checked() == checked
            && item
            .due()
            .datetime()
            .is_some_and(|dt| dt.month() == date.month() && dt.year() == date.year())
    }

    pub async fn get_items_by_overdeue_view(&self, checked: bool) -> Result<Vec<items::Model>, TodoError> {
        let now = Local::now().naive_local();
        let date_util = DateTime::default();
        Ok(Items::find().filter().all(&self.db).await?)

        self.items()
            .iter()
            .filter(|i| {
                i.has_due()
                    && !i.was_archived()
                    && i.checked()
                    && i.due()
                        .datetime()
                        .is_some_and(|dt| dt < now && !date_util.is_same_day(&dt, &now))
            })
            .cloned()
            .collect()
    }

    pub async fn get_all_items_by_search(
        &self,
        search_text: &str,
    ) -> Result<Vec<items::Model>, TodoError> {
        let search_lover = search_text.to_lowercase();
        Ok(Items::find()
            .filter(
                items::Column::Content
                    .contains(&search_lover)
                    .or(items::Column::Description.contains(&search_lover)),
            )
            .all(&self.db)
            .await?)
    }

    pub async fn valid_item_by_overdue(&self, item_id: &str, checked: bool) -> bool {
        let now = Local::now().naive_local();
        let date_util = DateTime::default();
        let item = self.get_item(item_id).await;
        if item.is_none() {
            return false;
        }
        let due = item.du
        let valid = item
            .due()
            .and_then(|due| due.datetime())
            .map(|dt| dt < now && date_util.is_same_day(&dt, &now))
            .unwrap_or_default();
        !(item.has_due() || item.was_archived()) && valid
    }

    // labels
    pub async fn labels(&self) -> Result<Vec<LabelModel>, TodoError> {
        Ok(Labels::find().all(&self.db).await?)
    }
    pub async fn insert_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        let mut active_label: labels::ActiveModel = label.into();
        Ok(active_label.insert(&self.db).await?)
    }
    pub async fn update_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        let mut active_label: labels::ActiveModel = label.into();
        Ok(active_label.update(&self.db).await?)
    }
    pub async fn delete_label(&self, id: &str) -> Result<u64, TodoError> {
        Ok(Labels::delete_by_id(id).exec(&self.db).await?.rows_affected)
    }
    pub async fn label_exists(&self, id: &str) -> Result<bool, TodoError> {
        Ok(Labels::find_by_id(id).one(&self.db).await?.is_some())
    }
    pub async fn get_label(&self, id: &str) -> Result<Option<LabelModel>, TodoError> {
        Ok(Labels::find_by_id(id).one(&self.db).await?)
    }
    pub async fn get_labels_by_item_labels(
        &self,
        labels: &str,
    ) -> Result<Vec<LabelModel>, TodoError> {
        let labels: Vec<String> = labels.split(',').map(|s| s.trim().to_string()).collect();
        Ok(Labels::find()
            .filter(labels::Column::Id.is_in(labels))
            .all(&self.db)
            .await?)
    }
    pub async fn get_label_by_name(
        &self,
        name: &str,
        source_id: &str,
    ) -> Result<Option<LabelModel>, TodoError> {
        Ok(Labels::find()
            .filter(
                labels::Column::Name
                    .eq(name)
                    .and(labels::Column::SourceId.eq(source_id)),
            )
            .one(&self.db)
            .await?)
    }
    pub async fn get_labels_by_source(
        &self,
        source_id: &str,
    ) -> Result<Vec<LabelModel>, TodoError> {
        Ok(Labels::find()
            .filter(labels::Column::SourceId.eq(source_id))
            .all(&self.db)
            .await?)
    }
    pub async fn get_all_labels_by_search(
        &self,
        search_text: &str,
    ) -> Result<Vec<LabelModel>, TodoError> {
        let search_lover = search_text.to_lowercase();
        Ok(Labels::find()
            .filter(labels::Column::Name.contains(&search_lover))
            .all(&self.db)
            .await?)
    }
    // reminders
    pub async fn reminders(&self) -> Result<Vec<reminders::Model>, TodoError> {
        Ok(Reminders::find().all(&self.db).await?)
    }
    pub async fn get_reminder(&self, id: &str) -> Result<Option<reminders::Model>, TodoError> {
        Ok(Reminders::find_by_id(id).one(&self.db).await?)
    }

    pub async fn get_reminders_by_item(
        &self,
        item_id: &str,
    ) -> Result<Vec<reminders::Model>, TodoError> {
        Ok(Reminders::find()
            .filter(reminders::Column::ItemId.eq(item_id))
            .all(&self.db)
            .await?)
    }
    pub async fn insert_reminder(
        &self,
        reminder: reminders::Model,
    ) -> Result<reminders::Model, TodoError> {
        let mut active_reminder: reminders::ActiveModel = reminder.into();
        Ok(active_reminder.insert(&self.db).await?)
        // reminder.item.reminder_added (reminder);
    }
    pub async fn delete_reminder(&self, reminder_id: &str) -> Result<DeleteResult, TodoError> {
        Ok(Reminders::delete_by_id(reminder_id).exec(&self.db).await?)
        // reminder.item.reminder_deleted (reminder);
    }
}
