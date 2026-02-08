//! Service Manager
//!
//! ServiceManager is a central coordinator for all services.
//! It manages the lifecycle of services and provides access to them.
//! Services can depend on each other through the ServiceManager.

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::{
    error::TodoError,
    services::{
        CacheManager, DateValidationService, EventBus, ItemService, LabelService, MetricsCollector,
        ProjectService, ReminderService, SectionService,
    },
};

/// Service Manager - Central coordinator for all services
#[derive(Clone, Debug)]
pub struct ServiceManager {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<EventBus>,
    cache: Arc<CacheManager>,
    metrics: Arc<MetricsCollector>,
    item_service: Arc<ItemService>,
    project_service: Arc<ProjectService>,
    section_service: Arc<SectionService>,
    label_service: Arc<LabelService>,
    reminder_service: Arc<ReminderService>,
    date_validation_service: Arc<DateValidationService>,
}

impl ServiceManager {
    /// Create a new ServiceManager
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let event_bus = Arc::new(EventBus::new());
        let cache = Arc::new(CacheManager::new(1000, 500, 500, 500, 500));
        let metrics = Arc::new(MetricsCollector::new());

        // Create services with dependencies
        // Note: Services are created in order of dependency
        let label_service = Arc::new(LabelService::new(
            db.clone(),
            event_bus.clone(),
            cache.clone(),
            metrics.clone(),
        ));

        let item_service = Arc::new(ItemService::new(
            db.clone(),
            event_bus.clone(),
            cache.clone(),
            metrics.clone(),
            label_service.clone(),
        ));

        let section_service = Arc::new(SectionService::new(
            db.clone(),
            event_bus.clone(),
            cache.clone(),
            metrics.clone(),
        ));

        let project_service = Arc::new(ProjectService::new(
            db.clone(),
            event_bus.clone(),
            cache.clone(),
            metrics.clone(),
            item_service.clone(),
            section_service.clone(),
        ));

        let reminder_service = Arc::new(ReminderService::new(
            db.clone(),
            event_bus.clone(),
            cache.clone(),
            metrics.clone(),
        ));

        let date_validation_service = Arc::new(DateValidationService::new(db.clone()));

        Self {
            db,
            event_bus,
            cache,
            metrics,
            item_service,
            project_service,
            section_service,
            label_service,
            reminder_service,
            date_validation_service,
        }
    }

    /// Get the database connection
    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    /// Get the event bus
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Get the cache manager
    pub fn cache(&self) -> &CacheManager {
        &self.cache
    }

    /// Get the metrics collector
    pub fn metrics(&self) -> &MetricsCollector {
        &self.metrics
    }

    /// Get the item service
    pub fn item_service(&self) -> &ItemService {
        &self.item_service
    }

    /// Get the project service
    pub fn project_service(&self) -> &ProjectService {
        &self.project_service
    }

    /// Get the section service
    pub fn section_service(&self) -> &SectionService {
        &self.section_service
    }

    /// Get the label service
    pub fn label_service(&self) -> &LabelService {
        &self.label_service
    }

    /// Get the reminder service
    pub fn reminder_service(&self) -> &ReminderService {
        &self.reminder_service
    }

    /// Get the date validation service
    pub fn date_validation_service(&self) -> &DateValidationService {
        &self.date_validation_service
    }

    /// Clear all caches
    pub async fn clear_caches(&self) {
        self.cache.clear_all().await;
    }
}
