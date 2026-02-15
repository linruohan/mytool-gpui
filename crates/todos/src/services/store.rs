//! Unified Store implementation using ServiceManager
//!
//! This module provides a unified Store implementation that delegates
//! operations to specialized services.
//!
//! It combines functionality from store.rs, store_new.rs, store_v2.rs and store_v3.rs
//! while removing code that has been moved to specialized service files.

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::{
    entity::{
        AttachmentModel, ItemModel, LabelModel, ProjectModel, ReminderModel, SectionModel,
        SourceModel,
    },
    error::TodoError,
    services::{
        DateValidationService, ItemService, LabelService, ProjectService, QueryService,
        ReminderService, SectionService, ServiceManager,
    },
};

/// Unified Store implementation using ServiceManager
#[derive(Clone, Debug)]
pub struct Store {
    service_manager: ServiceManager,
    item_service: ItemService,
    project_service: ProjectService,
    section_service: SectionService,
    label_service: LabelService,
    reminder_service: ReminderService,
    query_service: QueryService,
    date_validation_service: DateValidationService,
}

impl Store {
    /// Create a new Store
    pub fn new(db: DatabaseConnection) -> Self {
        let db = Arc::new(db);
        let service_manager = ServiceManager::new(db.clone());
        let query_service = QueryService::new(db.clone(), 10); // Max 10 concurrent queries

        Self {
            item_service: (*service_manager.item_service()).clone(),
            project_service: (*service_manager.project_service()).clone(),
            section_service: (*service_manager.section_service()).clone(),
            label_service: (*service_manager.label_service()).clone(),
            reminder_service: (*service_manager.reminder_service()).clone(),
            date_validation_service: (*service_manager.date_validation_service()).clone(),
            service_manager,
            query_service,
        }
    }

    /// Get the service manager
    pub fn service_manager(&self) -> &ServiceManager {
        &self.service_manager
    }

    /// Get the event bus
    pub fn event_bus(&self) -> &crate::services::EventBus {
        self.service_manager.event_bus()
    }

    /// Get the metrics collector
    pub fn metrics(&self) -> &crate::services::MetricsCollector {
        self.service_manager.metrics()
    }

    /// Get the database connection
    pub fn db(&self) -> &DatabaseConnection {
        self.service_manager.db()
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

    pub async fn move_item(
        &self,
        item_id: &str,
        project_id: &str,
        section_id: &str,
    ) -> Result<(), TodoError> {
        self.item_service.move_item(item_id, project_id, section_id).await
    }

    pub async fn complete_item(
        &self,
        item_id: &str,
        checked: bool,
        complete_sub_items: bool,
    ) -> Result<(), TodoError> {
        self.item_service.complete_item(item_id, checked, complete_sub_items).await
    }

    // ==================== Project Operations ====================

    pub async fn get_project(&self, id: &str) -> Option<ProjectModel> {
        self.project_service.get_project(id).await
    }

    pub async fn insert_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        self.project_service.insert_project(project).await
    }

    pub async fn update_project(&self, project: ProjectModel) -> Result<ProjectModel, TodoError> {
        self.project_service.update_project(project).await
    }

    pub async fn delete_project(&self, id: &str) -> Result<(), TodoError> {
        self.project_service.delete_project(id).await
    }

    pub async fn archive_project(&self, project_id: &str) -> Result<(), TodoError> {
        self.project_service.archive_project(project_id).await
    }

    pub async fn move_project(&self, project_id: &str, parent_id: &str) -> Result<(), TodoError> {
        self.project_service.move_project(project_id, parent_id).await
    }

    // ==================== Section Operations ====================

    pub async fn get_section(&self, id: &str) -> Option<SectionModel> {
        self.section_service.get_section(id).await
    }

    pub async fn insert_section(&self, section: SectionModel) -> Result<SectionModel, TodoError> {
        self.section_service.insert_section(section).await
    }

    pub async fn update_section(&self, section: SectionModel) -> Result<SectionModel, TodoError> {
        self.section_service.update_section(section).await
    }

    pub async fn delete_section(&self, section_id: &str) -> Result<(), TodoError> {
        self.section_service.delete_section(section_id).await
    }

    pub async fn move_section(&self, section_id: &str, project_id: &str) -> Result<(), TodoError> {
        self.section_service.move_section(section_id, project_id).await
    }

    pub async fn archive_section(&self, section_id: &str, archived: bool) -> Result<(), TodoError> {
        self.section_service.archive_section(section_id, archived).await
    }

    // ==================== Label Operations ====================

    pub async fn get_label(&self, id: &str) -> Option<LabelModel> {
        self.label_service.get_label(id).await
    }

    pub async fn insert_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        self.label_service.insert_label(label).await
    }

    pub async fn update_label(&self, label: LabelModel) -> Result<LabelModel, TodoError> {
        self.label_service.update_label(label).await
    }

    pub async fn delete_label(&self, id: &str) -> Result<u64, TodoError> {
        self.label_service.delete_label(id).await
    }

    pub async fn get_or_create_label(
        &self,
        name: &str,
        source_id: &str,
    ) -> Result<LabelModel, TodoError> {
        self.label_service.get_or_create_label(name, source_id).await
    }

    pub async fn get_labels_by_source(
        &self,
        source_id: &str,
    ) -> Result<Vec<LabelModel>, TodoError> {
        self.label_service.get_labels_by_source(source_id).await
    }

    // ==================== Additional Operations ====================

    // Item additional operations
    pub async fn get_items_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_items_by_project(project_id).await
    }

    pub async fn get_items_by_section(
        &self,
        section_id: &str,
    ) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_items_by_section(section_id).await
    }

    pub async fn get_subitems(&self, item_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_subitems(item_id).await
    }

    pub async fn get_pinned_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_pinned_items().await
    }

    pub async fn get_incomplete_pinned_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_incomplete_pinned_items().await
    }

    pub async fn get_completed_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_completed_items().await
    }

    pub async fn get_incomplete_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_incomplete_items().await
    }

    pub async fn get_scheduled_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_scheduled_items().await
    }

    pub async fn search_items(&self, search_text: &str) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.search_items(search_text).await
    }

    pub async fn archive_item(&self, item_id: &str, archived: bool) -> Result<(), TodoError> {
        self.item_service.archive_item(item_id, archived).await
    }

    pub async fn duplicate_item(&self, item_id: &str) -> Result<ItemModel, TodoError> {
        self.item_service.duplicate_item(item_id).await
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

    pub async fn get_items_by_label(&self, label_id: &str) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_items_by_label(label_id).await
    }

    pub async fn set_due_date(
        &self,
        item_id: &str,
        due_date: Option<chrono::NaiveDateTime>,
    ) -> Result<(), TodoError> {
        self.item_service.set_due_date(item_id, due_date).await
    }

    pub async fn get_items_due_today(&self) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_items_due_today().await
    }

    pub async fn get_overdue_items(&self) -> Result<Vec<ItemModel>, TodoError> {
        self.item_service.get_overdue_items().await
    }

    // Project additional operations
    pub async fn get_all_projects(&self) -> Result<Vec<ProjectModel>, TodoError> {
        self.project_service.get_all_projects().await
    }

    pub async fn get_projects_by_source(
        &self,
        source_id: &str,
    ) -> Result<Vec<ProjectModel>, TodoError> {
        self.project_service.get_projects_by_source(source_id).await
    }

    pub async fn get_subprojects(&self, parent_id: &str) -> Result<Vec<ProjectModel>, TodoError> {
        self.project_service.get_subprojects(parent_id).await
    }

    pub async fn get_archived_projects(&self) -> Result<Vec<ProjectModel>, TodoError> {
        self.project_service.get_archived_projects().await
    }

    pub async fn search_projects(&self, search_text: &str) -> Result<Vec<ProjectModel>, TodoError> {
        self.project_service.search_projects(search_text).await
    }

    pub async fn duplicate_project(&self, project_id: &str) -> Result<ProjectModel, TodoError> {
        self.project_service.duplicate_project(project_id).await
    }

    pub async fn get_project_stats(
        &self,
        project_id: &str,
    ) -> Result<crate::services::ProjectStats, TodoError> {
        self.project_service.get_project_stats(project_id).await
    }

    // Section additional operations
    pub async fn get_all_sections(&self) -> Result<Vec<SectionModel>, TodoError> {
        self.section_service.get_all_sections().await
    }

    pub async fn get_sections_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<SectionModel>, TodoError> {
        self.section_service.get_sections_by_project(project_id).await
    }

    pub async fn get_archived_sections(&self) -> Result<Vec<SectionModel>, TodoError> {
        self.section_service.get_archived_sections().await
    }

    pub async fn search_sections(&self, search_text: &str) -> Result<Vec<SectionModel>, TodoError> {
        self.section_service.search_sections(search_text).await
    }

    pub async fn duplicate_section(&self, section_id: &str) -> Result<SectionModel, TodoError> {
        self.section_service.duplicate_section(section_id).await
    }

    pub async fn get_section_stats(
        &self,
        section_id: &str,
    ) -> Result<crate::services::SectionStats, TodoError> {
        self.section_service.get_section_stats(section_id).await
    }

    // Label additional operations
    pub async fn get_all_labels(&self) -> Result<Vec<LabelModel>, TodoError> {
        self.label_service.get_all_labels().await
    }

    pub async fn search_labels(&self, search_text: &str) -> Result<Vec<LabelModel>, TodoError> {
        self.label_service.search_labels(search_text).await
    }

    pub async fn get_labels_by_item(&self, item_id: &str) -> Result<Vec<LabelModel>, TodoError> {
        self.label_service.get_labels_by_item(item_id).await
    }

    pub async fn add_label_to_item_by_id(
        &self,
        label_id: &str,
        item_id: &str,
    ) -> Result<(), TodoError> {
        self.label_service.add_label_to_item(label_id, item_id).await
    }

    pub async fn remove_label_from_item_by_id(
        &self,
        label_id: &str,
        item_id: &str,
    ) -> Result<(), TodoError> {
        self.label_service.remove_label_from_item(label_id, item_id).await
    }

    pub async fn get_label_stats(
        &self,
        label_id: &str,
    ) -> Result<crate::services::LabelStats, TodoError> {
        self.label_service.get_label_stats(label_id).await
    }

    pub async fn merge_labels(
        &self,
        source_label_id: &str,
        target_label_id: &str,
    ) -> Result<(), TodoError> {
        self.label_service.merge_labels(source_label_id, target_label_id).await
    }

    // ==================== Query Operations ====================

    /// Batch load items by IDs with concurrency control
    pub async fn batch_load_items(&self, ids: Vec<String>) -> Result<Vec<ItemModel>, TodoError> {
        self.query_service.batch_load_items(ids).await
    }

    /// Batch load projects by IDs with concurrency control
    pub async fn batch_load_projects(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<ProjectModel>, TodoError> {
        self.query_service.batch_load_projects(ids).await
    }

    /// Batch load sections by IDs with concurrency control
    pub async fn batch_load_sections(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<SectionModel>, TodoError> {
        self.query_service.batch_load_sections(ids).await
    }

    /// Batch load labels by IDs with concurrency control
    pub async fn batch_load_labels(&self, ids: Vec<String>) -> Result<Vec<LabelModel>, TodoError> {
        self.query_service.batch_load_labels(ids).await
    }

    // ==================== Reminder Operations ====================

    /// Get all reminders
    pub async fn reminders(&self) -> Result<Vec<ReminderModel>, TodoError> {
        self.reminder_service.get_all_reminders().await
    }

    /// Get a reminder by ID
    pub async fn get_reminder(&self, id: &str) -> Option<ReminderModel> {
        self.reminder_service.get_reminder(id).await
    }

    /// Get reminders by item ID
    pub async fn get_reminders_by_item(
        &self,
        item_id: &str,
    ) -> Result<Vec<ReminderModel>, TodoError> {
        self.reminder_service.get_reminders_by_item(item_id).await
    }

    /// Insert a new reminder
    pub async fn insert_reminder(
        &self,
        reminder: ReminderModel,
    ) -> Result<ReminderModel, TodoError> {
        self.reminder_service.insert_reminder(reminder).await
    }

    /// Update a reminder
    pub async fn update_reminder(
        &self,
        reminder: ReminderModel,
    ) -> Result<ReminderModel, TodoError> {
        self.reminder_service.update_reminder(reminder).await
    }

    /// Delete a reminder
    pub async fn delete_reminder(&self, reminder_id: &str) -> Result<u64, TodoError> {
        self.reminder_service.delete_reminder(reminder_id).await
    }

    /// Get reminders due before a specific time
    pub async fn get_reminders_due_before(
        &self,
        due_time: &chrono::NaiveDateTime,
    ) -> Result<Vec<ReminderModel>, TodoError> {
        self.reminder_service.get_reminders_due_before(due_time).await
    }

    /// Get reminders due after a specific time
    pub async fn get_reminders_due_after(
        &self,
        due_time: &chrono::NaiveDateTime,
    ) -> Result<Vec<ReminderModel>, TodoError> {
        self.reminder_service.get_reminders_due_after(due_time).await
    }

    /// Get reminders in a time range
    pub async fn get_reminders_in_range(
        &self,
        start_time: &chrono::NaiveDateTime,
        end_time: &chrono::NaiveDateTime,
    ) -> Result<Vec<ReminderModel>, TodoError> {
        self.reminder_service.get_reminders_in_range(start_time, end_time).await
    }

    // ==================== Date Validation Operations ====================

    /// Validate if an item matches a specific date
    pub async fn valid_item_by_date(
        &self,
        item_id: &str,
        date: &chrono::NaiveDateTime,
        checked: bool,
    ) -> bool {
        self.date_validation_service.valid_item_by_date(item_id, date, checked).await
    }

    /// Validate if an item matches a date range
    pub async fn valid_item_by_date_range(
        &self,
        item_id: &str,
        start_date: &chrono::NaiveDateTime,
        end_date: &chrono::NaiveDateTime,
        checked: bool,
    ) -> bool {
        self.date_validation_service
            .valid_item_by_date_range(item_id, start_date, end_date, checked)
            .await
    }

    /// Validate if an item matches a specific month
    pub async fn valid_item_by_month(
        &self,
        item_id: &str,
        date: &chrono::NaiveDateTime,
        checked: bool,
    ) -> bool {
        self.date_validation_service.valid_item_by_month(item_id, date, checked).await
    }

    /// Validate if an item is overdue
    pub async fn valid_item_by_overdue(&self, item_id: &str, checked: bool) -> bool {
        self.date_validation_service.valid_item_by_overdue(item_id, checked).await
    }
}
