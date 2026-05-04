//! Event bus system for centralized event handling
//!
//! This module provides an event bus system that allows components to publish and subscribe to
//! events. It includes support for automatic subscription cancellation when subscriptions are
//! dropped.
//!
//! ## 优化特性
//! - **泛型事件支持**: 支持任意类型的事件负载
//! - **类型安全**: 编译时检查事件类型
//! - **向后兼容**: 保留原有 Event 枚举

use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock, broadcast};
use uuid::Uuid;

// ==================== 泛型事件系统 ====================

/// 泛型事件 trait
///
/// 实现此 trait 的类型可以作为事件发布到 EventBus
pub trait EventTrait: Clone + Send + Sync + 'static {}

/// 泛型事件包装器
#[derive(Clone, Debug)]
struct EventWrapper {
    type_id: TypeId,
    payload: Arc<dyn std::any::Any + Send + Sync>,
}

/// 泛型订阅
pub struct TypedSubscription<T: EventTrait> {
    rx: broadcast::Receiver<T>,
    _event_bus: GenericEventBus,
}

impl<T: EventTrait> TypedSubscription<T> {
    /// 接收下一个事件
    pub async fn recv(&mut self) -> Result<T, broadcast::error::RecvError> {
        self.rx.recv().await
    }
}

/// 泛型事件总线
///
/// 支持发布和订阅不同类型的事件
#[derive(Clone, Debug)]
pub struct GenericEventBus {
    channels: Arc<RwLock<HashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>>>,
}

impl GenericEventBus {
    /// 创建新的泛型事件总线
    pub fn new() -> Self {
        Self { channels: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// 订阅特定类型的事件
    pub async fn subscribe<T: EventTrait>(&self) -> TypedSubscription<T> {
        let type_id = TypeId::of::<T>();
        let mut channels = self.channels.write().await;

        let sender =
            channels.entry(type_id).or_insert_with(|| Box::new(broadcast::channel::<T>(100).0));

        let tx = sender.downcast_ref::<broadcast::Sender<T>>().expect("Type mismatch in event bus");

        TypedSubscription { rx: tx.subscribe(), _event_bus: self.clone() }
    }

    /// 发布事件
    pub async fn publish<T: EventTrait>(&self, event: T) {
        let type_id = TypeId::of::<T>();
        let channels = self.channels.read().await;

        if let Some(sender) = channels.get(&type_id) {
            if let Some(tx) = sender.downcast_ref::<broadcast::Sender<T>>() {
                let _ = tx.send(event);
            }
        }
    }
}

impl Default for GenericEventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== 原有事件系统（向后兼容） ====================

/// Event types for the event bus
///
/// This enum defines all possible events that can be published and subscribed to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    /// Event fired when an item is created
    ///
    /// The string parameter is the ID of the created item.
    ItemCreated(String),
    /// Event fired when an item is updated
    ///
    /// The string parameter is the ID of the updated item.
    ItemUpdated(String),
    /// Event fired when an item is deleted
    ///
    /// The string parameter is the ID of the deleted item.
    ItemDeleted(String),
    /// Event fired when a project is created
    ///
    /// The string parameter is the ID of the created project.
    ProjectCreated(String),
    /// Event fired when a project is updated
    ///
    /// The string parameter is the ID of the updated project.
    ProjectUpdated(String),
    /// Event fired when a project is deleted
    ///
    /// The string parameter is the ID of the deleted project.
    ProjectDeleted(String),
    /// Event fired when a section is created
    ///
    /// The string parameter is the ID of the created section.
    SectionCreated(String),
    /// Event fired when a section is updated
    ///
    /// The string parameter is the ID of the updated section.
    SectionUpdated(String),
    /// Event fired when a section is deleted
    ///
    /// The string parameter is the ID of the deleted section.
    SectionDeleted(String),
    /// Event fired when a label is created
    ///
    /// The string parameter is the ID of the created label.
    LabelCreated(String),
    /// Event fired when a label is updated
    ///
    /// The string parameter is the ID of the updated label.
    LabelUpdated(String),
    /// Event fired when a label is deleted
    ///
    /// The string parameter is the ID of the deleted label.
    LabelDeleted(String),
    /// Event fired when a reminder is created
    ///
    /// The string parameter is the ID of the created reminder.
    ReminderCreated(String),
    /// Event fired when a reminder is updated
    ///
    /// The string parameter is the ID of the updated reminder.
    ReminderUpdated(String),
    /// Event fired when a reminder is deleted
    ///
    /// The string parameter is the ID of the deleted reminder.
    ReminderDeleted(String),
    /// Event fired when an attachment is created
    ///
    /// The string parameter is the ID of the created attachment.
    AttachmentCreated(String),
    /// Event fired when an attachment is updated
    ///
    /// The string parameter is the ID of the updated attachment.
    AttachmentUpdated(String),
    /// Event fired when an attachment is deleted
    ///
    /// The string parameter is the ID of the deleted attachment.
    AttachmentDeleted(String),
    /// Event fired when items position is updated
    ///
    /// The string parameters are the project ID and section ID respectively.
    ItemsPositionUpdated(String, String),
}

impl EventTrait for Event {}

// ==================== 便捷事件类型 ====================

/// 任务事件（泛型事件示例）
#[derive(Clone, Debug)]
pub struct ItemEvent {
    pub item_id: String,
    pub event_type: ItemEventType,
}

#[derive(Clone, Debug, Copy)]
pub enum ItemEventType {
    Created,
    Updated,
    Deleted,
}

impl EventTrait for ItemEvent {}

/// 项目事件（泛型事件示例）
#[derive(Clone, Debug)]
pub struct ProjectEvent {
    pub project_id: String,
    pub event_type: ProjectEventType,
}

#[derive(Clone, Debug, Copy)]
pub enum ProjectEventType {
    Created,
    Updated,
    Deleted,
}

impl EventTrait for ProjectEvent {}

/// Subscription to the event bus
///
/// This struct wraps a broadcast receiver and provides automatic cancellation when dropped.
pub struct Subscription {
    rx: broadcast::Receiver<Event>,
    event_bus: EventBus,
    listener_id: Option<Uuid>,
}

impl Drop for Subscription {
    /// Automatically cancel the subscription when dropped
    ///
    /// This ensures that resources are properly cleaned up and prevents memory leaks.
    fn drop(&mut self) {
        // Auto-cancel the subscription when dropped
        if let Some(listener_id) = self.listener_id {
            let event_bus = self.event_bus.clone();
            tokio::spawn(async move {
                event_bus.remove_listener(listener_id).await;
            });
        }
    }
}

impl Subscription {
    /// Create a new subscription
    ///
    /// # Parameters
    /// - `rx`: The broadcast receiver for events
    /// - `event_bus`: The event bus instance
    /// - `listener_id`: Optional listener ID for tracking
    ///
    /// # Returns
    /// A new Subscription instance
    pub fn new(
        rx: broadcast::Receiver<Event>,
        event_bus: EventBus,
        listener_id: Option<Uuid>,
    ) -> Self {
        Self { rx, event_bus, listener_id }
    }

    /// Receive the next event
    ///
    /// # Returns
    /// - `Ok(Event)`: The next event
    /// - `Err(broadcast::RecvError)`: Error receiving event
    pub async fn recv(&mut self) -> Result<Event, tokio::sync::broadcast::error::RecvError> {
        self.rx.recv().await
    }
}

/// Event bus for publishing and subscribing to events
///
/// This struct provides a centralized way to publish events and subscribe to them.
#[derive(Clone, Debug)]
pub struct EventBus {
    tx: Arc<broadcast::Sender<Event>>,
    listeners: Arc<Mutex<HashSet<Uuid>>>, // Store listener IDs instead of receivers
}

impl EventBus {
    /// Create a new event bus
    ///
    /// # Returns
    /// A new EventBus instance
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self { tx: Arc::new(tx), listeners: Arc::new(Mutex::new(HashSet::new())) }
    }

    /// Subscribe to events
    ///
    /// # Returns
    /// A broadcast receiver for events
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    /// Subscribe to events with automatic cancellation
    ///
    /// The subscription will be automatically canceled when dropped.
    ///
    /// # Returns
    /// A Subscription instance with automatic cancellation
    pub fn subscribe_auto_cancel(&self) -> Subscription {
        let rx = self.tx.subscribe();
        Subscription::new(rx, self.clone(), None)
    }

    /// Subscribe to events with automatic cancellation and listener tracking
    ///
    /// The subscription will be automatically canceled when dropped.
    ///
    /// # Returns
    /// A Subscription instance with automatic cancellation
    pub async fn subscribe_with_auto_cancel(&self) -> Subscription {
        let rx = self.tx.subscribe();
        let listener_id = Uuid::new_v4();
        let mut listeners = self.listeners.lock().await;
        listeners.insert(listener_id);
        Subscription::new(rx, self.clone(), Some(listener_id))
    }

    /// Publish an event
    ///
    /// # Parameters
    /// - `event`: The event to publish
    pub fn publish(&self, event: Event) {
        let _ = self.tx.send(event);
    }

    /// Add a listener to the event bus
    ///
    /// # Parameters
    /// - `rx`: The broadcast receiver for events
    pub async fn add_listener(&self, _rx: broadcast::Receiver<Event>) -> Uuid {
        let listener_id = Uuid::new_v4();
        let mut listeners = self.listeners.lock().await;
        listeners.insert(listener_id);
        listener_id
    }

    /// Remove a listener from the event bus
    ///
    /// # Parameters
    /// - `id`: The ID of the listener to remove
    pub async fn remove_listener(&self, id: Uuid) {
        let mut listeners = self.listeners.lock().await;
        listeners.remove(&id);
    }
}

impl Default for EventBus {
    /// Create a default event bus
    ///
    /// # Returns
    /// A new EventBus instance
    fn default() -> Self {
        Self::new()
    }
}
