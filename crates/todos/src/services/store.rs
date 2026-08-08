//! Unified Store facade over domain services
//!
//! Thin passthrough for GUI/cold-start hot paths only. Prefer specialized
//! services for new call sites.

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::{
    app::PatchManager,
    entity::{AttachmentModel, ItemModel, LabelModel, ProjectModel, ReminderModel, SectionModel},
    error::TodoError,
    services::{
        AttachmentService, ItemService, LabelService, ProjectService, ReminderService,
        SectionService,
    },
};

/// Unified Store implementation holding domain services directly
#[derive(Clone, Debug)]
pub struct Store {
    item_service: ItemService,
    project_service: ProjectService,
    section_service: SectionService,
    label_service: LabelService,
    reminder_service: ReminderService,
    attachment_service: AttachmentService,
}

impl Store {
    /// Create a new Store
    pub async fn new(db: DatabaseConnection) -> Result<Arc<Self>, TodoError> {
        let db = Arc::new(db);

        let patch_manager = PatchManager::new(db.clone());
        patch_manager.apply_patches().await?;

        let label_service = LabelService::new(db.clone());
        let label_service_for_item = Arc::new(label_service.clone());
        let item_service = ItemService::new(db.clone(), label_service_for_item);
        let item_service_for_deps = Arc::new(item_service.clone());
        let section_service =
            SectionService::new(db.clone(), item_service_for_deps.clone());
        let project_service = ProjectService::new(db.clone(), item_service_for_deps);
        let reminder_service = ReminderService::new(db.clone());
        let attachment_service = AttachmentService::new(db.clone());

        Ok(Arc::new(Self {
            item_service,
            project_service,
            section_service,
            label_service,
            reminder_service,
            attachment_service,
        }))
    }

    // ==================== Item Operations ====================

    pub async fn get_item(&self, id: &str) -> Option<ItemModel> {
        self.item_service.get_item(id).await
    }

    pub async fn insert_item(&self, item: ItemModel, insert: bool) -> Result<ItemModel, TodoError> {
        self.item_service.insert_item(item, insert).await
    }

    pub async fn update_item(
        &self,
        item: ItemModel,
        update_id: &str,
    ) -> Result<ItemModel, TodoError> {
        self.item_service.update_item(item, update_id).await
    }

    pub async fn delete_item(&self, item_id: &str) -> Result<(), TodoError> {
        self.item_service.delete_item(item_id).await
    }

    pub async fn update_item_pin(&self, item_id: &str, pinned: bool) -> Result<(), TodoError> {
        self.item_service.update_item_pin(item_id, pinned).await
    }

    pub async fn complete_item(
        &self,
        item_id: &str,
        checked: bool,
        complete_sub_items: bool,
    ) -> Result<(), TodoError> {
        self.item_service.complete_item(item_id, checked, complete_sub_items).await
    }

    pub async fn get_all_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_all_items().await
    }

    pub async fn get_items_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_items_by_project(project_id).await
    }

    pub async fn add_label_to_item(
        &self,
        item_id: &str,
        label_name: &str,
    ) -> Result<(), TodoError> {
        self.item_service.add_label_to_item(item_id, label_name).await
    }

    pub async fn remove_label_from_item(
        &self,
        item_id: &str,
        label_id: &str,
    ) -> Result<(), TodoError> {
        self.item_service.remove_label_from_item(item_id, label_id).await
    }

    pub async fn set_item_labels(
        &self,
        item_id: &str,
        label_ids: &[String],
    ) -> Result<(), TodoError> {
        self.item_service.set_item_labels(item_id, label_ids).await
    }

    pub async fn get_labels_by_item(&self, item_id: &str) -> Result<Vec<LabelModel>, TodoError> {
        self.item_service.get_labels_by_item(item_id).await
    }

    // ==================== Project Operations ====================

    pub async fn insert_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        self.project_service.insert_project(project).await
    }

    pub async fn update_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        self.project_service.update_project(project).await
    }

    pub async fn delete_project(&self, id: &str) -> Result<(), TodoError> {
        self.project_service.delete_project(id).await
    }

    pub async fn get_all_projects(&self) -> Result<Vec<ProjectModel>, TodoError> {
        self.project_service.get_all_projects().await
    }

    // ==================== Section Operations ====================

    pub async fn insert_section(&self, section: SectionModel) -> Result<SectionModel, TodoError> {
        self.section_service.insert_section(section).await
    }

    pub async fn update_section(&self, section: SectionModel) -> Result<SectionModel, TodoError> {
        self.section_service.update_section(section).await
    }

    pub async fn delete_section(&self, section_id: &str) -> Result<(), TodoError> {
        self.section_service.delete_section(section_id).await
    }

    pub async fn get_all_sections(&self) -> Result<Vec<SectionModel>, TodoError> {
        self.section_service.get_all_sections().await
    }

    // ==================== Label Operations ====================

    pub async fn insert_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        self.label_service.insert_label(label).await
    }

    pub async fn update_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        self.label_service.update_label(label).await
    }

    pub async fn delete_label(&self, id: &str) -> Result<u64, TodoError> {
        self.label_service.delete_label(id).await
    }

    pub async fn get_all_labels(&self) -> Result<Vec<LabelModel>, TodoError> {
        self.label_service.get_all_labels().await
    }

    // ==================== Reminder Operations ====================

    pub async fn get_reminders_by_item(
        &self,
        item_id: &str,
    ) -> Result<Vec<ReminderModel>, TodoError> {
        self.reminder_service.get_reminders_by_item(item_id).await
    }

    pub async fn insert_reminder(
        &self,
        reminder: ReminderModel,
    ) -> Result<ReminderModel, TodoError> {
        self.reminder_service.insert_reminder(reminder).await
    }

    pub async fn delete_reminder(&self, reminder_id: &str) -> Result<u64, TodoError> {
        self.reminder_service.delete_reminder(reminder_id).await
    }

    // ==================== Attachment Operations ====================

    pub async fn get_attachments_by_item(
        &self,
        item_id: &str,
    ) -> Result<Vec<AttachmentModel>, TodoError> {
        self.attachment_service.get_attachments_by_item(item_id).await
    }

    pub async fn insert_attachment(
        &self,
        attachment: AttachmentModel,
    ) -> Result<AttachmentModel, TodoError> {
        self.attachment_service.insert_attachment(attachment).await
    }

    pub async fn delete_attachment(&self, attachment_id: &str) -> Result<u64, TodoError> {
        self.attachment_service.delete_attachment(attachment_id).await
    }

    // ==================== Batch Operations ====================

    pub async fn batch_update_items(
        &self,
        items: Vec<ItemModel>,
    ) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.batch_update_items(items).await
    }
}
