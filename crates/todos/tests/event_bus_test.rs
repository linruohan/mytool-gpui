#[cfg(test)]
mod tests {
    use todos::services::event_bus::{Event, EventBus};

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let event_bus = EventBus::new();
        let mut rx = event_bus.subscribe();

        event_bus.publish(Event::ItemCreated("test-id".to_string()));

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, Event::ItemCreated(id) if id == "test-id"));
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let event_bus = EventBus::new();
        let mut rx1 = event_bus.subscribe();
        let mut rx2 = event_bus.subscribe();

        event_bus.publish(Event::ItemUpdated("test-id".to_string()));

        let event1 = rx1.recv().await.unwrap();
        let event2 = rx2.recv().await.unwrap();

        assert!(matches!(event1, Event::ItemUpdated(_)));
        assert!(matches!(event2, Event::ItemUpdated(_)));
    }

    #[tokio::test]
    async fn test_subscribe_auto_cancel() {
        let event_bus = EventBus::new();

        {
            let _subscription = event_bus.subscribe_auto_cancel();
            // Subscription 存在
        }
        // Subscription 自动取消，不应 panic
    }

    #[tokio::test]
    async fn test_subscribe_with_auto_cancel() {
        let event_bus = EventBus::new();

        let subscription = event_bus.subscribe_with_auto_cancel().await;
        // Subscription 存在
        drop(subscription);
        // Subscription 自动取消，不应 panic
    }

    #[tokio::test]
    async fn test_event_types() {
        let event_bus = EventBus::new();
        let mut rx = event_bus.subscribe();

        // 测试所有事件类型
        event_bus.publish(Event::ItemCreated("1".to_string()));
        event_bus.publish(Event::ItemUpdated("2".to_string()));
        event_bus.publish(Event::ItemDeleted("3".to_string()));
        event_bus.publish(Event::ProjectCreated("4".to_string()));
        event_bus.publish(Event::SectionCreated("5".to_string()));
        event_bus.publish(Event::LabelCreated("6".to_string()));
        event_bus.publish(Event::ReminderCreated("7".to_string()));
        event_bus.publish(Event::AttachmentCreated("8".to_string()));
        event_bus.publish(Event::ItemsPositionUpdated("9".to_string(), "10".to_string()));

        // 验证事件按顺序接收
        assert!(matches!(rx.recv().await.unwrap(), Event::ItemCreated(_)));
        assert!(matches!(rx.recv().await.unwrap(), Event::ItemUpdated(_)));
        assert!(matches!(rx.recv().await.unwrap(), Event::ItemDeleted(_)));
    }
}
