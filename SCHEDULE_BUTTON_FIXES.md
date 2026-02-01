# Schedule Button 数据同步修复

## 问题分析

在 `schedule_button.rs` 中，date_picker 的初始化、数据同步和数据保存存在以下问题：

1. **NumberInput 事件未订阅** - `recurrency_interval_input` 和 `recurrency_count_input` 的值变化没有被捕获
2. **输入值未同步到状态** - 用户输入的值没有更新到 `recurrency_interval` 和 `recurrency_count` 字段
3. **Popover 打开/关闭时数据不同步** - 打开时没有初始化输入框，关闭时没有保存数据

## 实施的修复

### 1. 添加 InputEvent 订阅

在 `new()` 方法中添加了两个新的事件订阅：

```rust
cx.subscribe_in(&recurrency_interval_input, window, Self::on_recurrency_interval_event),
cx.subscribe_in(&recurrency_count_input, window, Self::on_recurrency_count_event),
```

### 2. 实现输入事件处理器

添加了两个新方法来处理输入值的变化：

- `on_recurrency_interval_event()` - 监听 recurrency_interval_input 的变化，解析值并更新 `recurrency_interval` 和 `due_date.recurrency_interval`
- `on_recurrency_count_event()` - 监听 recurrency_count_input 的变化，解析值并更新 `recurrency_count` 和 `due_date.recurrency_count`

### 3. 增强 Popover 打开/关闭逻辑

在按钮点击处理中：

**打开 Popover 时：**
- 同步 date_picker 的日期
- 同步 recurrency_date_picker 的日期（如果需要）
- **新增：** 同步 NumberInput 的值到输入框

**关闭 Popover 时：**
- **新增：** 确保 `recurrency_interval` 和 `recurrency_count` 同步到 `due_date`

### 4. 增强 sync_from_due_date 方法

在从数据库加载数据时，现在也会同步 NumberInput 的值：

```rust
self.recurrency_interval_input.update(cx, |input, ctx| {
    input.set_value(self.recurrency_interval.to_string(), window, ctx);
});
self.recurrency_count_input.update(cx, |input, ctx| {
    input.set_value(self.recurrency_count.to_string(), window, ctx);
});
```

## 数据流程

### 初始化流程
1. `ItemInfoState::new()` 创建 `ScheduleButtonState`
2. 如果 item 有 due_date，调用 `sync_from_due_date()`
3. `sync_from_due_date()` 同步所有字段，包括 NumberInput 的值

### 用户交互流程
1. 用户打开 Popover → 同步所有输入框的值
2. 用户修改 NumberInput → 触发 `on_recurrency_interval_event()` 或 `on_recurrency_count_event()`
3. 事件处理器更新 `recurrency_interval`/`recurrency_count` 和 `due_date` 中的对应字段
4. 用户关闭 Popover → 确保数据同步到 `due_date`
5. `on_schedule_event()` 在 `ItemInfoState` 中被触发
6. `ItemInfoState` 将 `due_date` 序列化为 JSON 并保存到 `item.due`
7. `update_item()` 将数据保存到数据库

### 数据加载流程
1. 打开 item 时，`ItemInfoState::set_item()` 被调用
2. 从 `item.due` 反序列化 `DueDate`
3. 调用 `schedule_button_state.set_due_date()` 或 `sync_from_due_date()`
4. 所有字段（包括 NumberInput）都被正确初始化
5. 再次打开 Popover 时，所有值都已正确同步

## 关键改进

✅ **完整的数据同步** - 所有输入字段在打开/关闭 Popover 时都会同步
✅ **事件驱动** - NumberInput 的值变化会立即更新到状态
✅ **数据持久化** - 关闭 Popover 时确保数据保存到 `due_date`
✅ **数据恢复** - 重新打开 item 时，所有数据都会正确恢复

## 测试步骤

1. 创建一个新的 todo item
2. 点击 Schedule 按钮打开 Popover
3. 设置日期、时间和重复规则
4. 在"Repeat every"输入框中输入数字（如 3）
5. 在"Occurrences"输入框中输入数字（如 5）
6. 关闭 Popover
7. 保存 item
8. 重新打开 item
9. 点击 Schedule 按钮
10. 验证所有值都被正确保留
