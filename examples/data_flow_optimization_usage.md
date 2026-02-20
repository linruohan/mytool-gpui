# 数据流优化使用示例

本文档展示如何使用新的数据流优化功能。

## 1. 乐观更新示例

### 添加任务

```rust
use crate::core::actions::add_item_optimistic;
use todos::entity::ItemModel;
use std::sync::Arc;

// 创建新任务
let new_item = ItemModel {
    id: String::new(), // 会自动生成临时 ID
    content: "完成报告".to_string(),
    checked: false,
    pinned: false,
    project_id: None,
    section_id: None,
    due: None,
    ..Default::default()
};

// 乐观添加（立即更新 UI）
add_item_optimistic(Arc::new(new_item), cx);
// UI 立即显示新任务，无需等待数据库操作
```

### 更新任务

```rust
use crate::core::actions::update_item_optimistic;

// 获取要更新的任务
let mut item = (*existing_item).clone();
item.content = "更新后的内容".to_string();

// 乐观更新（立即更新 UI）
update_item_optimistic(Arc::new(item), cx);
// UI 立即显示更新，无需等待数据库操作
```

### 完成任务

```rust
use crate::core::actions::complete_item_optimistic;

// 完成任务
complete_item_optimistic(item.clone(), true, cx);
// UI 立即显示为已完成

// 取消完成
complete_item_optimistic(item.clone(), false, cx);
// UI 立即显示为未完成
```

### 删除任务

```rust
use crate::core::actions::delete_item_optimistic;

// 删除任务
delete_item_optimistic(item.clone(), cx);
// UI 立即移除任务
```

## 2. 缓存查询示例

### 使用缓存查询收件箱任务

```rust
use crate::core::state::{TodoStore, QueryCache};

// 获取 store 和 cache
let store = cx.global::<TodoStore>();
let cache = cx.global::<QueryCache>();

// 使用缓存查询（如果缓存有效，直接返回）
let inbox_items = store.inbox_items_cached(cache);

// 不使用缓存（每次都重新计算）
let inbox_items = store.inbox_items();
```

### 使用缓存查询今日任务

```rust
let store = cx.global::<TodoStore>();
let cache = cx.global::<QueryCache>();

// 使用缓存
let today_items = store.today_items_cached(cache);
```

### 手动清空缓存

```rust
use crate::core::state::QueryCache;

let cache = cx.global::<QueryCache>();

// 清空所有缓存
cache.invalidate_all();

// 清空特定项目的缓存
cache.invalidate_project("project_id");

// 清空特定分区的缓存
cache.invalidate_section("section_id");
```

## 3. 事件总线示例

### 发布事件

```rust
use crate::core::state::{TodoEventBus, TodoStoreEvent};
use gpui::BorrowAppContext;

// 发布任务添加事件
cx.update_global::<TodoEventBus, _>(|bus, _| {
    bus.publish(TodoStoreEvent::ItemAdded("item_id".to_string()));
});

// 发布任务更新事件
cx.update_global::<TodoEventBus, _>(|bus, _| {
    bus.publish(TodoStoreEvent::ItemUpdated("item_id".to_string()));
});

// 发布批量更新事件
cx.update_global::<TodoEventBus, _>(|bus, _| {
    bus.publish(TodoStoreEvent::BulkUpdate);
});
```

### 查看事件历史

```rust
use crate::core::state::TodoEventBus;

let bus = cx.global::<TodoEventBus>();

// 获取最近 10 个事件
let recent_events = bus.recent_events(10);

for event in recent_events {
    println!("Event: {:?}", event);
}
```

## 4. 批量操作示例

### 收集批量操作

```rust
use crate::core::state::BatchOperations;
use gpui::BorrowAppContext;

// 添加到批量操作队列
cx.update_global::<BatchOperations, _>(|ops, _| {
    ops.add_item(item1.clone());
    ops.add_item(item2.clone());
    ops.update_item(item3.clone());
});

// 检查是否有待处理的操作
let has_pending = cx.global::<BatchOperations>().has_pending;
let count = cx.global::<BatchOperations>().pending_count();
```

### 批量提交（未来实现）

```rust
// 注意：批量提交功能需要进一步实现
// 这里只是展示预期的 API

use crate::core::state::BatchOperations;

// 获取待处理的操作
let ops = cx.global::<BatchOperations>();
let adds = ops.pending_adds.clone();
let updates = ops.pending_updates.clone();
let deletes = ops.pending_deletes.clone();

// 批量提交到数据库
if !adds.is_empty() {
    state_service::batch_add_items(
        adds.iter().map(|i| (**i).clone()).collect(),
        db.clone()
    ).await?;
}

// 清空队列
cx.update_global::<BatchOperations, _>(|ops, _| {
    ops.clear();
});
```

## 5. 在视图中使用

### 在 Board 视图中使用缓存

```rust
impl InboxBoard {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut base = BoardBase::new(window, cx);
        
        base._subscriptions = vec![
            cx.observe_global_in::<TodoStore>(window, move |this, window, cx| {
                let store = cx.global::<TodoStore>();
                let cache = cx.global::<QueryCache>();
                
                // 使用缓存查询
                let state_items = store.inbox_items_cached(cache);
                
                // 更新视图
                this.update_view(state_items, window, cx);
                cx.notify();
            }),
        ];
        
        Self { base }
    }
}
```

### 在组件中使用乐观更新

```rust
impl ItemRow {
    fn on_complete_clicked(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let item = self.item.clone();
        
        // 使用乐观更新
        complete_item_optimistic(item, true, cx);
        
        // UI 立即更新，无需等待
    }
    
    fn on_delete_clicked(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let item = self.item.clone();
        
        // 使用乐观更新
        delete_item_optimistic(item, cx);
        
        // UI 立即更新，无需等待
    }
}
```

## 6. 性能对比

### 传统方式（同步更新）

```rust
// 添加任务 - 需要等待数据库操作
add_item(item, cx);
// 用户需要等待 100-200ms 才能看到新任务
```

### 乐观更新方式

```rust
// 添加任务 - 立即更新 UI
add_item_optimistic(item, cx);
// 用户立即看到新任务（< 10ms）
// 数据库操作在后台异步进行
```

### 缓存查询方式

```rust
// 不使用缓存 - 每次都重新计算
let items = store.inbox_items(); // 10-20ms

// 使用缓存 - 如果缓存有效，直接返回
let cache = cx.global::<QueryCache>();
let items = store.inbox_items_cached(cache); // < 1ms（缓存命中）
```

## 7. 最佳实践

### 何时使用乐观更新

✅ **推荐使用**:
- 用户频繁操作的功能（添加、完成、删除任务）
- 需要快速响应的交互
- 网络延迟较高的环境

❌ **不推荐使用**:
- 批量操作（使用批量 API）
- 需要立即确认的操作
- 数据一致性要求极高的场景

### 何时使用缓存

✅ **推荐使用**:
- 频繁查询的数据（收件箱、今日任务）
- 计算成本较高的查询
- 数据变化不频繁的场景

❌ **不推荐使用**:
- 实时性要求极高的数据
- 数据变化非常频繁
- 内存受限的环境

### 错误处理

```rust
// 乐观更新会自动处理错误和回滚
// 但你可以添加额外的错误提示

use crate::core::actions::add_item_optimistic;

// 添加任务
add_item_optimistic(item.clone(), cx);

// 如果需要，可以监听事件总线来处理错误
// （未来可以添加错误事件类型）
```

## 8. 调试技巧

### 查看事件历史

```rust
// 在调试时查看最近的事件
let bus = cx.global::<TodoEventBus>();
let events = bus.recent_events(20);

for event in events {
    tracing::debug!("Event: {:?}", event);
}
```

### 检查缓存状态

```rust
// 检查缓存是否有效
let cache = cx.global::<QueryCache>();
let store = cx.global::<TodoStore>();
let is_valid = cache.is_valid(store.version());

tracing::debug!("Cache valid: {}", is_valid);
```

### 监控性能

```rust
use std::time::Instant;

// 测量查询性能
let start = Instant::now();
let items = store.inbox_items();
let duration = start.elapsed();
tracing::debug!("Query took: {:?}", duration);

// 测量缓存查询性能
let start = Instant::now();
let items = store.inbox_items_cached(cache);
let duration = start.elapsed();
tracing::debug!("Cached query took: {:?}", duration);
```

## 9. 迁移指南

### 从传统方式迁移到乐观更新

**步骤 1**: 找到使用传统方式的代码

```rust
// 旧代码
use crate::core::actions::add_item;
add_item(item, cx);
```

**步骤 2**: 替换为乐观更新

```rust
// 新代码
use crate::core::actions::add_item_optimistic;
add_item_optimistic(item, cx);
```

**步骤 3**: 测试功能是否正常

- 测试正常流程（添加、更新、删除）
- 测试错误流程（网络断开、数据库错误）
- 验证回滚是否正常工作

### 从普通查询迁移到缓存查询

**步骤 1**: 找到频繁查询的代码

```rust
// 旧代码
let items = cx.global::<TodoStore>().inbox_items();
```

**步骤 2**: 添加缓存查询

```rust
// 新代码
let store = cx.global::<TodoStore>();
let cache = cx.global::<QueryCache>();
let items = store.inbox_items_cached(cache);
```

**步骤 3**: 验证性能提升

- 使用性能监控工具测量查询时间
- 验证缓存命中率
- 确保数据一致性

## 10. 常见问题

### Q: 乐观更新失败后，UI 没有回滚？

**A**: 检查以下几点：
1. 确保 `TodoEventBus` 已正确初始化
2. 确保视图正确订阅了 `TodoStore` 的变化
3. 查看日志中的错误信息

### Q: 缓存没有生效？

**A**: 检查以下几点：
1. 确保使用 `*_cached` 方法
2. 确保 `QueryCache` 已正确初始化
3. 验证版本号是否正确更新

### Q: 如何禁用乐观更新？

**A**: 使用传统的同步更新方法：
```rust
// 使用传统方式（同步更新）
use crate::core::actions::add_item;
add_item(item, cx);
```

---

**更新日期**: 2026-02-20  
**维护者**: Kiro AI Assistant
