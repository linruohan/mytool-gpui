//! Service Manager
//!
//! ServiceManager is a central coordinator for all services.
//! It manages the lifecycle of services and provides access to them.
//! Services can depend on each other through the ServiceManager.

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::{
    app::{DatabaseManager, PatchManager, TransactionManager},
    error::TodoError,
    services::{
        DateValidationService, EventBus, EventRecorder, ItemService, LabelService,
        MetricsCollector, ProjectService, ReminderService, SectionService,
    },
};

/// Service Manager - Central coordinator for all services
#[derive(Clone, Debug)]
pub struct ServiceManager {
    db: Arc<DatabaseConnection>,
    database_manager: Arc<DatabaseManager>,
    transaction_manager: Arc<TransactionManager>,
    event_recorder: Arc<EventRecorder>,
    event_bus: Arc<EventBus>,
    metrics: Arc<MetricsCollector>,
    item_service: Arc<ItemService>,
    project_service: Arc<ProjectService>,
    section_service: Arc<SectionService>,
    label_service: Arc<LabelService>,
    reminder_service: Arc<ReminderService>,
    date_validation_service: Arc<DateValidationService>,
    // 用于跟踪是否已经应用过补丁
    patches_applied: bool,
}

impl ServiceManager {
    /// Create a new ServiceManager
    pub async fn new(db: Arc<DatabaseConnection>) -> Result<Self, TodoError> {
        let event_bus = Arc::new(EventBus::new());
        let metrics = Arc::new(MetricsCollector::new());

        // 🚀 关键修复：使用 try_read 避免死锁
        // 如果无法获取锁，panic 并提示可能的死锁问题
        let database_config = gconfig::get()
            .try_read()
            .expect(
                "Failed to acquire gconfig read lock - possible deadlock detected! Please report \
                 this issue.",
            )
            .database()
            .clone();

        // Create new components
        let database_manager = Arc::new(DatabaseManager::new(database_config).await?);

        let transaction_manager = Arc::new(TransactionManager::new(db.clone()));

        let event_recorder = Arc::new(EventRecorder::new(db.clone()));

        // Apply database patches
        let patch_manager = PatchManager::new(db.clone());
        patch_manager.apply_patches().await?;

        // Create services with dependencies
        // Note: Services are created in order of dependency
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

        let date_validation_service = Arc::new(DateValidationService::new(db.clone()));

        Ok(Self {
            db,
            database_manager,
            transaction_manager,
            event_recorder,
            event_bus,
            metrics,
            item_service,
            project_service,
            section_service,
            label_service,
            reminder_service,
            date_validation_service,
            patches_applied: false,
        })
    }

    /// Get the database connection
    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    /// Get the event bus
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
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

    /// Get the database manager
    pub fn database_manager(&self) -> &DatabaseManager {
        &self.database_manager
    }

    /// Get the transaction manager
    pub fn transaction_manager(&self) -> &TransactionManager {
        &self.transaction_manager
    }

    /// Get the patch manager
    /// Note: PatchManager is not stored in ServiceManager anymore, this method is deprecated
    pub fn patch_manager(&self) -> &PatchManager {
        panic!("PatchManager is not stored in ServiceManager anymore")
    }

    /// Get the event recorder
    pub fn event_recorder(&self) -> &EventRecorder {
        &self.event_recorder
    }
}
