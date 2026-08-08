//! Service Manager
//!
//! Central coordinator for all services. Manages lifecycle and dependency wiring.
#![allow(dead_code)]

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::{
    app::PatchManager,
    error::TodoError,
    services::{
        AttachmentService, EventBus, ItemService, LabelService, MetricsCollector, ProjectService,
        ReminderService, SectionService,
    },
};

/// Service Manager - Central coordinator for all services
#[derive(Clone, Debug)]
pub struct ServiceManager {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
    metrics: Arc<MetricsCollector>,
    item_service: Arc<ItemService>,
    project_service: Arc<ProjectService>,
    section_service: Arc<SectionService>,
    label_service: Arc<LabelService>,
    reminder_service: Arc<ReminderService>,
    attachment_service: Arc<AttachmentService>,
}

impl ServiceManager {
    /// Create a new ServiceManager
    pub async fn new(db: Arc<DatabaseConnection>) -> Result<Self, TodoError> {
        let event_bus = Arc::new(EventBus::new());
        let metrics = Arc::new(MetricsCollector::new());

        // Apply database patches
        let patch_manager = PatchManager::new(db.clone());
        patch_manager.apply_patches().await?;

        let label_service =
            Arc::new(LabelService::new(db.clone(), event_bus.clone(), metrics.clone()));

        let item_service = Arc::new(ItemService::new(
            db.clone(),
            event_bus.clone(),
            metrics.clone(),
            label_service.clone(),
        ));
        let section_service = Arc::new(SectionService::new(
            db.clone(),
            event_bus.clone(),
            metrics.clone(),
            item_service.clone(),
        ));

        let project_service = Arc::new(ProjectService::new(
            db.clone(),
            event_bus.clone(),
            metrics.clone(),
            item_service.clone(),
            section_service.clone(),
        ));

        let reminder_service =
            Arc::new(ReminderService::new(db.clone(), event_bus.clone(), metrics.clone()));

        let attachment_service =
            Arc::new(AttachmentService::new(db.clone(), event_bus.clone(), metrics.clone()));

        Ok(Self {
            db,
            event_bus,
            metrics,
            item_service,
            project_service,
            section_service,
            label_service,
            reminder_service,
            attachment_service,
        })
    }

    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    pub fn metrics(&self) -> &MetricsCollector {
        &self.metrics
    }

    pub fn item_service(&self) -> &ItemService {
        &self.item_service
    }

    pub fn project_service(&self) -> &ProjectService {
        &self.project_service
    }

    pub fn section_service(&self) -> &SectionService {
        &self.section_service
    }

    pub fn label_service(&self) -> &LabelService {
        &self.label_service
    }

    pub fn reminder_service(&self) -> &ReminderService {
        &self.reminder_service
    }

    pub fn attachment_service(&self) -> &AttachmentService {
        &self.attachment_service
    }
}
