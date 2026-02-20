# 批量操作使用指南

## 概述

批量操作功能允许一次性处理多个任务，大幅减少数据库往返次数，提升应用性能。

## 性能对比

### 传统方式（单个操作）
```rust
// 添加 100 个任务 = 100 次数据库操作
for item in items {
    add_item(item, cx);  // 每次都是独立的数据库操作
}
// 总耗时: ~1000ms (假设每次 10ms)
```

### 批量操作方式
```rust
// 添加 100 个任务 = 1 次批量操作
batch_add_items(items, cx);
// 总耗时: ~50ms (批量处理)
// 性能提升: 20倍
```

## 可用的批量操作

### 1. 批量添加任务

```rust
use crate::todo_actions::batch_add_items;

// 准备要添加的任务
let items = vec![
    Arc::new(ItemModel {
        content: "任务 1".to_string(),
        ..Default::default()
    }),
    Arc::new(ItemModel {
        content: "任务 2".to_string(),
        ..Default::default()
    }),
    // ... 更多任务
];

// 批量添加
batch_add_items(items, cx);
```

**使用场景**:
- 导入任务列表
- 从模板创建多个任务
- 批量复制任务

### 2. 批量更新任务

```rust
use crate::todo_actions::batch_update_items;

// 准备要更新的任务
let items = vec![
    Arc::new(updated_item1),
    Arc::new(updated_item2),
    // ... 更多任务
];

// 批量更新
batch_update_items(items, cx);
```

**使用场景**:
- 批量修改任务属性（如优先级、标签）
- 批量移动任务到其他项目
- 批量设置截止日期

### 3. 批量删除任务

```rust
use crate::todo_actions::batch_delete_items;

// 准备要删除的任务 ID
let item_ids = vec![
    "item_id_1".to_string(),
    "item_id_2".to_string(),
    // ... 更多 ID
];

// 批量删除
batch_delete_items(item_ids, cx);
```

**使用场景**:
- 清空已完成任务
- 删除过期任务
- 批量删除选中的任务

### 4. 批量完成/取消完成任务

```rust
use crate::todo_actions::batch_complete_items;

// 准备要完成的任务 ID
let item_ids = vec![
    "item_id_1".to_string(),
    "item_id_2".to_string(),
    // ... 更多 ID
];

// 批量完成任务
batch_complete_items(item_ids.clone(), true, cx);

// 批量取消完成
batch_complete_items(item_ids, false, cx);
```

**使用场景**:
- 一键完成所有今日任务
- 批量标记任务为未完成
- 完成项目中的所有任务

## 批量操作队列（高级用法）

对于需要防抖或延迟提交的场景，可以使用 `BatchQueue`：

```rust
use crate::todo_actions::{BatchQueue, flush_batch_queue};

// 创建队列
let mut queue = BatchQueue::new();

// 收集操作
queue.pending_adds.push(Arc::new(item1));
queue.pending_adds.push(Arc::new(item2));
queue.pending_updates.push(Arc::new(updated_item));
queue.pending_deletes.push("item_id_to_delete".to_string());

// 在合适的时机刷新队列（例如 300ms 后）
cx.spawn_after(Duration::from_millis(300), |cx| async move {
    let db = cx.global::<DBState>().conn.clone();
    flush_batch_queue(&mut queue, cx, db).await;
}).detach();
```

**使用场景**:
- 用户快速连续操作时，合并多个操作
- 自动保存功能（定时刷新队列）
- 离线模式（收集操作，联网后批量同步）

## 错误处理

批量操作采用"部分失败"策略：

```rust
// 即使某些任务失败，其他任务仍会继续处理
batch_add_items(items, cx);
// 结果：
// - 成功的任务会被添加到 TodoStore
// - 失败的任务会记录错误日志
// - 不会因为一个失败而中断整个批量操作
```

查看日志：
```bash
# 查看批量操作日志
grep "Batch" logs/app.log

# 示例输出：
# INFO: Batch adding 100 items
# INFO: Successfully added 98 items in batch
# ERROR: Failed to insert item in batch: DatabaseError(...)
```

## 性能建议

### 1. 批量大小

```rust
// 推荐：每批 50-200 个任务
const BATCH_SIZE: usize = 100;

// 如果任务数量很大，分批处理
for chunk in items.chunks(BATCH_SIZE) {
    batch_add_items(chunk.to_vec(), cx);
}
```

### 2. 何时使用批量操作

✅ **适合使用批量操作**:
- 导入/导出功能
- 批量编辑
- 数据同步
- 清理操作

❌ **不适合使用批量操作**:
- 单个任务的增删改查（使用普通操作即可）
- 需要立即反馈的操作
- 操作之间有依赖关系

### 3. 与版本号机制配合

批量操作会自动更新 TodoStore 的版本号：

```rust
// 批量添加 100 个任务
batch_add_items(items, cx);

// TodoStore 版本号只增加 1 次
// 所有观察者只触发 1 次更新
// 而不是 100 次更新
```

这与版本号缓存机制完美配合，进一步提升性能。

## 实际应用示例

### 示例 1: 导入任务列表

```rust
pub fn import_tasks_from_file(file_path: &str, cx: &mut App) {
    cx.spawn(async move |cx| {
        // 1. 读取文件
        let content = tokio::fs::read_to_string(file_path).await?;
        
        // 2. 解析任务
        let items: Vec<ItemModel> = serde_json::from_str(&content)?;
        let arc_items: Vec<Arc<ItemModel>> = items.into_iter()
            .map(Arc::new)
            .collect();
        
        // 3. 批量导入
        batch_add_items(arc_items, cx);
        
        Ok(())
    }).detach();
}
```

### 示例 2: 清空已完成任务

```rust
pub fn clear_completed_tasks(cx: &mut App) {
    let store = cx.global::<TodoStore>();
    
    // 获取所有已完成任务的 ID
    let completed_ids: Vec<String> = store.all_items
        .iter()
        .filter(|item| item.checked)
        .map(|item| item.id.clone())
        .collect();
    
    // 批量删除
    if !completed_ids.is_empty() {
        batch_delete_items(completed_ids, cx);
    }
}
```

### 示例 3: 批量设置优先级

```rust
pub fn set_priority_for_selected(selected_ids: Vec<String>, priority: Priority, cx: &mut App) {
    let store = cx.global::<TodoStore>();
    
    // 准备更新的任务
    let updated_items: Vec<Arc<ItemModel>> = selected_ids.iter()
        .filter_map(|id| {
            store.all_items.iter()
                .find(|item| &item.id == id)
                .map(|item| {
                    let mut updated = (**item).clone();
                    updated.priority = priority;
                    Arc::new(updated)
                })
        })
        .collect();
    
    // 批量更新
    if !updated_items.is_empty() {
        batch_update_items(updated_items, cx);
    }
}
```

## 监控和调试

### 启用详细日志

```rust
// 在 main.rs 中设置日志级别
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();
```

### 性能指标

批量操作会自动记录性能指标：

```
INFO: Batch adding 100 items
INFO: Successfully added 100 items in batch (took 45ms)
```

### 调试技巧

```rust
// 1. 检查队列状态
let queue = cx.global::<BatchQueue>();
println!("Pending operations: {}", queue.total_operations());

// 2. 手动刷新队列（用于测试）
flush_batch_queue(&mut queue, cx, db).await;

// 3. 验证结果
let store = cx.global::<TodoStore>();
println!("Total items: {}", store.all_items.len());
```

## 总结

批量操作是提升应用性能的关键优化之一：

- ✅ 减少数据库往返次数（20倍性能提升）
- ✅ 与版本号机制完美配合
- ✅ 支持部分失败，提高可靠性
- ✅ 提供队列机制，支持防抖优化
- ✅ 完整的错误处理和日志记录

建议在所有涉及多个任务操作的场景中使用批量操作。
