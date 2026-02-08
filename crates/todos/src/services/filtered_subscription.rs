//! Filtered subscription for event bus

use std::sync::Arc;

use crate::services::event_bus::{Event, Subscription};

/// Filtered subscription with event filtering capability
pub struct FilteredSubscription {
    inner: Subscription,
    filter: Arc<dyn Fn(&Event) -> bool + Send + Sync>,
}

impl FilteredSubscription {
    /// Create a new filtered subscription
    ///
    /// # Parameters
    /// - `subscription`: The underlying subscription
    /// - `filter`: Filter function that returns true for events to receive
    pub fn new(
        subscription: Subscription,
        filter: Arc<dyn Fn(&Event) -> bool + Send + Sync>,
    ) -> Self {
        Self { inner: subscription, filter }
    }

    /// Receive the next filtered event
    ///
    /// # Returns
    /// - `Ok(Event)`: The next event that passes the filter
    /// - `Err(broadcast::RecvError)`: Error receiving event
    pub async fn recv(&mut self) -> Result<Event, tokio::sync::broadcast::error::RecvError> {
        loop {
            let event = self.inner.recv().await?;
            if (self.filter)(&event) {
                return Ok(event);
            }
        }
    }

    /// Create a filter for item events
    pub fn filter_item_events(subscription: Subscription, item_id: String) -> Self {
        Self::new(
            subscription,
            Arc::new(move |event| {
                matches!(event,
                    Event::ItemCreated(id) | Event::ItemUpdated(id) | Event::ItemDeleted(id)
                    if id == &item_id
                )
            }),
        )
    }

    /// Create a filter for project events
    pub fn filter_project_events(subscription: Subscription, project_id: String) -> Self {
        Self::new(
            subscription,
            Arc::new(move |event| {
                matches!(event,
                    Event::ProjectCreated(id) | Event::ProjectUpdated(id) | Event::ProjectDeleted(id)
                    if id == &project_id
                )
            }),
        )
    }

    /// Create a filter for section events
    pub fn filter_section_events(subscription: Subscription, section_id: String) -> Self {
        Self::new(
            subscription,
            Arc::new(move |event| {
                matches!(event,
                    Event::SectionCreated(id) | Event::SectionUpdated(id) | Event::SectionDeleted(id)
                    if id == &section_id
                )
            }),
        )
    }

    /// Create a filter for label events
    pub fn filter_label_events(subscription: Subscription, label_id: String) -> Self {
        Self::new(
            subscription,
            Arc::new(move |event| {
                matches!(event,
                    Event::LabelCreated(id) | Event::LabelUpdated(id) | Event::LabelDeleted(id)
                    if id == &label_id
                )
            }),
        )
    }

    /// Create a filter for reminder events
    pub fn filter_reminder_events(subscription: Subscription, reminder_id: String) -> Self {
        Self::new(
            subscription,
            Arc::new(move |event| {
                matches!(event,
                    Event::ReminderCreated(id) | Event::ReminderUpdated(id) | Event::ReminderDeleted(id)
                    if id == &reminder_id
                )
            }),
        )
    }

    /// Create a filter for attachment events
    pub fn filter_attachment_events(subscription: Subscription, attachment_id: String) -> Self {
        Self::new(
            subscription,
            Arc::new(move |event| {
                matches!(event,
                    Event::AttachmentCreated(id) | Event::AttachmentDeleted(id)
                    if id == &attachment_id
                )
            }),
        )
    }

    /// Create a filter for item position updates
    pub fn filter_position_updates(
        subscription: Subscription,
        project_id: String,
        section_id: String,
    ) -> Self {
        Self::new(
            subscription,
            Arc::new(move |event| {
                matches!(event,
                    Event::ItemsPositionUpdated(proj_id, sec_id)
                    if proj_id == &project_id && sec_id == &section_id
                )
            }),
        )
    }
}
