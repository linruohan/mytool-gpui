//! Cache management for todos
//!
//! This module provides a centralized caching mechanism for todos entities
//! to reduce database queries and improve performance.

use std::{num::NonZeroUsize, sync::Arc};

use lru::LruCache;
use tokio::sync::RwLock;

use crate::{
    entity::{ItemModel, LabelModel, ProjectModel, ReminderModel, SectionModel},
    error::TodoError,
};

/// Cache manager for todos entities
#[derive(Clone, Debug)]
pub struct CacheManager {
    item_cache: Arc<RwLock<LruCache<String, ItemModel>>>,
    project_cache: Arc<RwLock<LruCache<String, ProjectModel>>>,
    section_cache: Arc<RwLock<LruCache<String, SectionModel>>>,
    label_cache: Arc<RwLock<LruCache<String, LabelModel>>>,
    reminder_cache: Arc<RwLock<LruCache<String, ReminderModel>>>,
}

impl CacheManager {
    /// Create a new cache manager with specified capacity for each cache
    pub fn new(
        item_capacity: usize,
        project_capacity: usize,
        section_capacity: usize,
        label_capacity: usize,
        reminder_capacity: usize,
    ) -> Self {
        Self {
            item_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(item_capacity).unwrap_or(NonZeroUsize::new(100).unwrap()),
            ))),
            project_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(project_capacity).unwrap_or(NonZeroUsize::new(50).unwrap()),
            ))),
            section_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(section_capacity).unwrap_or(NonZeroUsize::new(50).unwrap()),
            ))),
            label_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(label_capacity).unwrap_or(NonZeroUsize::new(100).unwrap()),
            ))),
            reminder_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(reminder_capacity).unwrap_or(NonZeroUsize::new(100).unwrap()),
            ))),
        }
    }

    /// Get or load an item from cache
    pub async fn get_or_load_item<F, Fut>(
        &self,
        id: &str,
        loader: F,
    ) -> Result<ItemModel, TodoError>
    where
        F: FnOnce(&str) -> Fut,
        Fut: std::future::Future<Output = Result<ItemModel, TodoError>>,
    {
        // Try to get from cache first
        {
            let mut cache = self.item_cache.write().await;
            if let Some(item) = cache.get(id) {
                return Ok(item.clone());
            }
        }

        // Load from database
        let item = loader(id).await?;

        // Store in cache
        {
            let mut cache = self.item_cache.write().await;
            cache.put(id.to_string(), item.clone());
        }

        Ok(item)
    }

    /// Get or load a project from cache
    pub async fn get_or_load_project<F, Fut>(
        &self,
        id: &str,
        loader: F,
    ) -> Result<ProjectModel, TodoError>
    where
        F: FnOnce(&str) -> Fut,
        Fut: std::future::Future<Output = Result<ProjectModel, TodoError>>,
    {
        {
            let mut cache = self.project_cache.write().await;
            if let Some(project) = cache.get(id) {
                return Ok(project.clone());
            }
        }

        let project = loader(id).await?;

        {
            let mut cache = self.project_cache.write().await;
            cache.put(id.to_string(), project.clone());
        }

        Ok(project)
    }

    /// Get or load a section from cache
    pub async fn get_or_load_section<F, Fut>(
        &self,
        id: &str,
        loader: F,
    ) -> Result<SectionModel, TodoError>
    where
        F: FnOnce(&str) -> Fut,
        Fut: std::future::Future<Output = Result<SectionModel, TodoError>>,
    {
        {
            let mut cache = self.section_cache.write().await;
            if let Some(section) = cache.get(id) {
                return Ok(section.clone());
            }
        }

        let section = loader(id).await?;

        {
            let mut cache = self.section_cache.write().await;
            cache.put(id.to_string(), section.clone());
        }

        Ok(section)
    }

    /// Get or load a label from cache
    pub async fn get_or_load_label<F, Fut>(
        &self,
        id: &str,
        loader: F,
    ) -> Result<LabelModel, TodoError>
    where
        F: FnOnce(&str) -> Fut,
        Fut: std::future::Future<Output = Result<LabelModel, TodoError>>,
    {
        {
            let mut cache = self.label_cache.write().await;
            if let Some(label) = cache.get(id) {
                return Ok(label.clone());
            }
        }

        let label = loader(id).await?;

        {
            let mut cache = self.label_cache.write().await;
            cache.put(id.to_string(), label.clone());
        }

        Ok(label)
    }

    /// Get or load a reminder from cache
    pub async fn get_or_load_reminder<F, Fut>(
        &self,
        id: &str,
        loader: F,
    ) -> Result<ReminderModel, TodoError>
    where
        F: FnOnce(&str) -> Fut,
        Fut: std::future::Future<Output = Result<ReminderModel, TodoError>>,
    {
        {
            let mut cache = self.reminder_cache.write().await;
            if let Some(reminder) = cache.get(id) {
                return Ok(reminder.clone());
            }
        }

        let reminder = loader(id).await?;

        {
            let mut cache = self.reminder_cache.write().await;
            cache.put(id.to_string(), reminder.clone());
        }

        Ok(reminder)
    }

    /// Invalidate item cache
    pub async fn invalidate_item(&self, id: &str) {
        let mut cache = self.item_cache.write().await;
        cache.pop(id);
    }

    /// Invalidate project cache
    pub async fn invalidate_project(&self, id: &str) {
        let mut cache = self.project_cache.write().await;
        cache.pop(id);
    }

    /// Invalidate section cache
    pub async fn invalidate_section(&self, id: &str) {
        let mut cache = self.section_cache.write().await;
        cache.pop(id);
    }

    /// Invalidate label cache
    pub async fn invalidate_label(&self, id: &str) {
        let mut cache = self.label_cache.write().await;
        cache.pop(id);
    }

    /// Invalidate reminder cache
    pub async fn invalidate_reminder(&self, id: &str) {
        let mut cache = self.reminder_cache.write().await;
        cache.pop(id);
    }

    /// Clear all caches
    pub async fn clear_all(&self) {
        let mut item_cache = self.item_cache.write().await;
        item_cache.clear();
        let mut project_cache = self.project_cache.write().await;
        project_cache.clear();
        let mut section_cache = self.section_cache.write().await;
        section_cache.clear();
        let mut label_cache = self.label_cache.write().await;
        label_cache.clear();
        let mut reminder_cache = self.reminder_cache.write().await;
        reminder_cache.clear();
    }
}
