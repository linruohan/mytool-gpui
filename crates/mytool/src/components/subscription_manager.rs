use gpui::Subscription;

/// 订阅管理器，统一管理组件的事件订阅
pub struct SubscriptionManager {
    subscriptions: Vec<Subscription>,
}

impl SubscriptionManager {
    /// 创建新的订阅管理器
    pub fn new() -> Self {
        Self { subscriptions: Vec::new() }
    }

    /// 添加订阅
    pub fn add(&mut self, subscription: Subscription) {
        self.subscriptions.push(subscription);
    }

    /// 获取所有订阅
    pub fn subscriptions(&self) -> &[Subscription] {
        &self.subscriptions
    }

    /// 清空所有订阅
    pub fn clear(&mut self) {
        self.subscriptions.clear();
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}
