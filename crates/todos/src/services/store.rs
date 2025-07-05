use crate::entity::{attachments, labels, prelude::*, projects, reminders, sections, sources};
use crate::error::TodoError;
use crate::utils::DateTime;
use chrono::{Datelike, Local, NaiveDateTime};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DeleteResult, EntityTrait, ModelTrait,
    QueryFilter, Set,
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
        Ok(Attachments::delete_by_id(id).exec(&self.db).await?.rows_affected)

        // attachment.deleted ();
        // attachment_deleted (attachment);
        // _attachments.remove (attachment);
        //
        // attachment.item.attachment_deleted (attachment);
    }

    pub async fn insert_attachment(
        &self,
        item_id: &str,
        file_type: Option<String>,
        file_name: &str,
        file_size: &str,
        file_path: &str,
    ) -> Result<attachments::Model, TodoError> {
        let attachment = attachments::ActiveModel {
            id: Default::default(),
            item_id: item_id.to_string().into(),
            file_type: file_type.into(),
            file_name: Default::default(),
            file_size: Default::default(),
            file_path: Default::default(),
        };
        let insert = attachment.insert(&self.db).await?;
        Ok(insert)
        // attachment.item.attachment_added (attachment);
    }

    pub async fn get_attachments_by_itemid(
        &self,
        item_id: &str,
    ) -> Result<Vec<attachments::Model>, TodoError> {
        Ok(Attachments::find().filter(attachments::Column::ItemId.eq(item_id)).all(&self.db).await?)
    }

    // sources
    pub async fn sources(&self) -> Result<Vec<sources::Model>, TodoError> {
        Ok(Sources::find().all(&self.db).await?)
    }
    pub async fn get_source(&self, id: &str) -> Result<sources::Model, TodoError> {
        Ok(Sources::find()
            .filter(sources::Column::Id.eq(id))
            .one(&self.db).await?.unwrap())
    }

    pub async fn insert_source(
        &self,
        source_type: &str,
        display_name: Option<String>,
        data: Option<String>,
    ) -> Result<sources::Model, TodoError> {
        let sources = self.sources().await?;
        let source = sources::ActiveModel {
            id: Default::default(),
            display_name: display_name.into(),
            source_type: source_type.into(),
            child_order: Some(sources.len() as i32 + 1).into(),
            ..Default::default()
        };
        Ok(source.insert(&self.db).await?)
    }
    pub async fn delete_source(&self, source_id: &str) {
        let source = Sources::find().filter(sources::Column::Id.eq(source_id)).one(&self.db).await?;
        if let Some(s) = source {
            source.delete(&self.db).await?;
        };
        // for project in self.get_projects_by_source(source.id()) {
        //     self.delete_project(&project);
        // }
    }

    pub async fn update_source(
        &self,
        source_id: &str,
        display_name: Option<String>,
        data: Option<String>,
    ) -> Result<sources::Model, TodoError> {
        let source = Sources::find().filter(sources::Column::Id.eq(source_id)).one(&self.db).await?;
        if let Some(mut s) = source {
            s.display_name = Set(display_name);
            s.data = Set(data);
            s.update(&self.db).await?;
            Ok(s)
        } else {
            Err(TodoError::NotFound("Source not found".to_string()))
        }
    }
    // projects
    pub async fn projects(&self) -> Result<Vec<projects::Model>, TodoError> {
        Ok(Projects::find().all(&self.db).await?)
    }
    pub async fn insert_project(&self, project: &Project) {
        let project = projects::ActiveModel {
            id: Default::default(),
            name: project.name.clone().into(),
            source_id: project.source_id.clone().into(),
            parent_id: project.parent_id.clone().into(),
            child_order: Some(project.child_order).into(),
            is_archived: Some(project.is_archived).into(),
            is_inbox_project: Some(project.is_inbox_project).into(),
            ..Default::default()
        };
        if Database::default().insert_project(project)
            && let Some(parent) = project.parent()
        {
            parent.add_subproject(project);
        }
    }
    pub async fn get_project(&self, id: &str) -> Option<Project> {
        self.projects()
            .iter()
            .find(|s| s.id.as_deref() == Some(id))
            .cloned()
    }
    pub async fn get_projects_by_source(&self, id: &str) -> Vec<Project> {
        self.projects()
            .iter()
            .filter(|s| s.source_id.as_deref() == Some(id))
            .cloned()
            .collect()
    }
    pub async fn update_project(&self, project: Project) {
        if Database::default().update_project(project.clone()) {
            // project.updated();
        }
    }
    pub async fn delete_project(&self, project: &Project) {
        let project_id = project.id_string();
        if Database::default().delete_project(project) {
            for section in self.get_sections_by_project(project) {
                self.delete_section(&section);
            }
            for item in self.get_items_by_project(project) {
                self.delete_item(&item);
            }
            for subproject in self.get_subprojects(project_id) {
                self.delete_project(&subproject);
            }
        }
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

    pub async fn archive_project(&self, project: &Project) {
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
    pub async fn get_sections_by_project(&self, project: &Project) -> Vec<Section> {
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
    pub async fn insert_section(&self, section: &Section) {
        if Database::default().insert_section(section) {
            // self.sections().push(section.clone());
            // section.project.section_added (section);
        }
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
    pub async fn get_item(&self, id: &str) -> Option<Item> {
        self.items()
            .iter()
            .find(|i| i.id.as_deref() == Some(id))
            .cloned()
    }

    pub async fn get_items_by_section(&self, id: &str) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|s| s.section_id.as_deref() == Some(id))
            .cloned()
            .collect()
    }

    pub async fn get_subitems(&self, item_id: &str) -> Vec<Item> {
        self.items()
            .iter().filter(|s| s.parent_id.as_deref() == Some(item_id))
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

    pub async fn get_items_by_label(&self, label_id: &str, checked: bool) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| i.has_label(label_id) && i.checked() == checked && !i.was_archived())
            .cloned()
            .collect()
    }
    pub async fn get_item_by_baseobject(&self, base: Box<dyn BaseTrait>) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|item| match (&item.project_id, &item.section_id) {
                // 项目过滤
                (Some(pid), None) => pid == base.id(),
                // 章节过滤
                (Some(_), Some(sid)) => sid == base.id(),
                _ => false,
            })
            .cloned()
            .collect()
    }
    pub async fn get_items_checked(&self) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| i.checked())
            .cloned()
            .collect()
    }
    pub async fn get_items_checked_by_project(&self, project: &Project) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| i.project_id == project.id && i.checked())
            .cloned()
            .collect()
    }
    pub async fn get_subitems_uncomplete(&self, item: &Item) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| i.parent_id == i.id && !i.checked())
            .cloned()
            .collect()
    }
    pub async fn get_items_by_project(&self, project: &Project) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| i.exists_project(project))
            .cloned()
            .collect()
    }
    pub async fn get_items_by_project_pinned(&self, project: &Project) -> Vec<Item> {
        self.items()
            .iter()
            .filter(|i| i.exists_project(project) && i.pinned())
            .cloned()
            .collect()
    }
    pub async fn get_items_by_date(&self, date: &NaiveDateTime, checked: bool) -> Vec<Item> {
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

    pub async fn get_items_by_overdeue_view(&self, checked: bool) -> Vec<Item> {
        let now = Local::now().naive_local();
        let date_util = DateTime::default();

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

    pub async fn get_all_items_by_search(&self, search_text: &str) -> Vec<Item> {
        let search_lower = search_text.to_lowercase();
        self.items()
            .iter()
            .filter(|i| {
                !i.checked()
                    && !i.was_archived()
                    && (i.content.to_lowercase().contains(&search_lower)
                        || i.description
                            .as_deref()
                            .map(|desc| desc.to_lowercase().contains(&search_lower))
                            .unwrap_or(false))
            })
            .cloned()
            .collect()
    }

    pub async fn valid_item_by_overdue(&self, item: Item, checked: bool) -> bool {
        let now = Local::now().naive_local();
        let date_util = DateTime::default();
        !(item.has_due() || item.was_archived())
            && item
                .due()
                .datetime()
                .is_some_and(|dt| dt <= now && date_util.is_same_day(&dt, &now))
    }

    // labels
    pub async fn labels(&self) -> Result<Vec<labels::Model>, TodoError> {
        Ok(Labels::find().all(&self.db).await?)
    }
    pub async fn insert_label(&self, label: Label) {
        if Database::default().insert_label(label) {
            todo!()
        }
    }
    pub async fn update_label(&self, label: Label) {
        if Database::default().update_label(label) {
            // label.updated ();
            // label_updated (label);
            todo!()
        }
    }
    pub async fn delete_label(&self, label: Label) {
        if Database::default().delete_label(label) {
            // label.deleted ();
            // label_deleted (label);
            // _labels.remove (label);
            todo!()
        }
    }
    pub async fn label_exists(&self, id: &str) -> bool {
        self.labels().iter().any(|s| s.id.as_deref() == Some(id))
    }
    pub async fn get_label(&self, id: &str) -> Option<Label> {
        self.labels()
            .iter()
            .find(|s| s.id.as_deref() == Some(id))
            .cloned()
    }
    pub async fn get_labels_by_item_labels(&self, labels: &str) -> Vec<Label> {
        labels
            .split(';')
            .filter_map(|id| self.get_label(id))
            .collect()
    }
    pub async fn get_label_by_name(&self, name: &str, source_id: &str) -> Option<Label> {
        self.labels()
            .iter()
            .find(|l| {
                l.name
                    .as_deref()
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
                    && l.source_id.as_deref() == Some(source_id)
            })
            .cloned()
    }
    pub async fn get_labels_by_source(&self, id: &str) -> Vec<Label> {
        self.labels()
            .iter()
            .filter(|l| l.source_id.as_deref() == Some(id))
            .cloned()
            .collect()
    }
    pub async fn get_all_labels_by_search(&self, search_text: &str) -> Vec<Label> {
        let search_lover = search_text.to_lowercase();
        self.labels()
            .iter()
            .filter(|s| {
                s.name
                    .as_deref()
                    .map(|name| name.contains(&search_lover))
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }
    // reminders
    pub async fn reminders(&self) -> Result<Vec<reminders::Model>, TodoError> {
        Ok(Reminders::find().all(&self.db).await?)
    }
    pub async fn get_reminder(&self, id: &str) -> Option<Reminder> {
        self.reminders()
            .iter()
            .find(|s| s.id.as_deref() == Some(id))
            .cloned()
    }

    pub async fn get_reminders_by_item(&self, item: &Item) -> Vec<Reminder> {
        self.reminders()
            .iter()
            .filter(|s| s.item_id == item.id)
            .cloned()
            .collect()
    }
    pub async fn insert_reminder(&self, reminder: &Reminder) {
        if Database::default().insert_reminder(reminder) {
            // reminders.add (reminder);
            // reminder_added (reminder);
            // reminder.item.reminder_added (reminder);
            todo!()
        }
    }
    pub async fn delete_reminder(&self, reminder_id: &str) -> Result<DeleteResult, TodoError> {
        Ok(Reminders::delete_by_id(reminder_id).exec(&self.db).await?)
        // reminder.item.reminder_deleted (reminder);
    }
}
