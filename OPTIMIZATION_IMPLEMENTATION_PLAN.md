# 优化实施计划

> 将已完成的优化应用到实际代码中的详细计划

## 📋 概述

本文档描述如何将已完成的 6 大优化应用到现有代码库中，确保优化效果得到充分发挥。

## ✅ 已完成的优化

1. ✅ 版本号追踪系统
2. ✅ 视图层版本号缓存
3. ✅ 批量操作系统
4. ✅ 键盘快捷键系统
5. ✅ 统一错误处理
6. ✅ 数据库连接优化

## 🎯 实施阶段

### 阶段 1: 错误处理应用（已开始）

**目标**: 在所有 todo_actions 中应用统一错误处理

**进度**: 🔄 进行中

**已完成**:
- ✅ `item.rs` - 已应用错误处理和输入验证

**待完成**:
- [ ] `project.rs` - 应用错误处理
- [ ] `label.rs` - 应用错误处理
- [ ] `section.rs` - 应用错误处理
- [ ] `reminder.rs` - 应用错误处理
- [ ] `attachment.rs` - 应用错误处理
- [ ] `batch_operations.rs` - 增强错误处理

**实施步骤**:

1. **添加输入验证**
   ```rust
   // 在每个操作前验证输入
   if let Err(e) = validation::validate_xxx(input) {
       let context = ErrorHandler::handle_with_location(e, "function_name");
       error!("{}", context.format_user_message());
       return;
   }
   ```

2. **统一错误处理**
   ```rust
   match operation().await {
       Ok(result) => {
           info!("Successfully completed: {}", result.id);
           // 处理成功...
       }
       Err(e) => {
           let context = ErrorHandler::handle_with_resource(
               AppError::Database(e),
               "function_name",
               &resource_id,
           );
           error!("{}", context.format_user_message());
           // TODO: 显示错误提示给用户
       }
   }
   ```

3. **添加日志记录**
   ```rust
   use tracing::{info, error};
   
   info!("Successfully added item: {}", item.id);
   error!("Failed to add item: {}", error);
   ```

---

### 阶段 2: 快捷键实现

**目标**: 实现快捷键处理逻辑

**进度**: ⏳ 待开始

**任务清单**:

1. **在主窗口注册快捷键**
   ```rust
   // 在 main.rs 或 lib.rs 中
   use crate::shortcuts::*;
   
   // 注册任务操作快捷键
   cx.on_action(|action: &NewTask, window, cx| {
       // 打开新建任务对话框
       show_new_task_dialog(window, cx);
   });
   
   cx.on_action(|action: &EditTask, window, cx| {
       // 编辑选中的任务
       edit_selected_task(window, cx);
   });
   
   // ... 注册其他快捷键
   ```

2. **实现快捷键处理函数**
   ```rust
   fn show_new_task_dialog(window: &mut Window, cx: &mut App) {
       // 实现新建任务对话框
   }
   
   fn edit_selected_task(window: &mut Window, cx: &mut App) {
       // 获取选中的任务
       // 打开编辑对话框
   }
   
   fn delete_selected_task(window: &mut Window, cx: &mut App) {
       // 获取选中的任务
       // 确认后删除
   }
   ```

3. **添加快捷键提示**
   ```rust
   // 在按钮和菜单项上显示快捷键
   Button::new("add-task")
       .label("新建任务")
       .tooltip("新建任务 (Cmd+N)")
       .on_click(|_, window, cx| {
           show_new_task_dialog(window, cx);
       })
   ```

4. **实现导航快捷键**
   ```rust
   cx.on_action(|action: &ShowInbox, window, cx| {
       // 切换到收件箱视图
       switch_to_view("inbox", window, cx);
   });
   
   cx.on_action(|action: &ShowToday, window, cx| {
       // 切换到今日任务视图
       switch_to_view("today", window, cx);
   });
   ```

---

### 阶段 3: 批量操作入口

**目标**: 在 UI 中添加批量操作入口

**进度**: ⏳ 待开始

**任务清单**:

1. **添加选择模式**
   ```rust
   pub struct TaskListState {
       selected_items: HashSet<String>,
       selection_mode: bool,
   }
   
   impl TaskListState {
       pub fn toggle_selection(&mut self, item_id: &str) {
           if self.selected_items.contains(item_id) {
               self.selected_items.remove(item_id);
           } else {
               self.selected_items.insert(item_id.to_string());
           }
       }
       
       pub fn select_all(&mut self, items: &[Arc<ItemModel>]) {
           self.selected_items = items.iter()
               .map(|item| item.id.clone())
               .collect();
       }
   }
   ```

2. **添加批量操作按钮**
   ```rust
   // 在工具栏添加批量操作按钮
   if !selected_items.is_empty() {
       h_flex()
           .gap_2()
           .child(
               Button::new("batch-complete")
                   .label(format!("完成 ({})", selected_items.len()))
                   .on_click(|_, _, cx| {
                       batch_complete_selected(cx);
                   })
           )
           .child(
               Button::new("batch-delete")
                   .label("删除")
                   .on_click(|_, _, cx| {
                       batch_delete_selected(cx);
                   })
           )
   }
   ```

3. **实现批量操作函数**
   ```rust
   fn batch_complete_selected(cx: &mut App) {
       let selected = get_selected_item_ids(cx);
       if !selected.is_empty() {
           batch_complete_items(selected, true, cx);
       }
   }
   
   fn batch_delete_selected(cx: &mut App) {
       let selected = get_selected_item_ids(cx);
       if !selected.is_empty() {
           // 显示确认对话框
           if confirm_delete(selected.len(), cx) {
               batch_delete_items(selected, cx);
           }
       }
   }
   ```

---

### 阶段 4: 性能监控

**目标**: 添加性能监控和基准测试

**进度**: ⏳ 待开始

**任务清单**:

1. **添加性能指标收集**
   ```rust
   pub struct PerformanceMetrics {
       render_count: usize,
       render_time: Duration,
       version_checks: usize,
       version_hits: usize,
   }
   
   impl PerformanceMetrics {
       pub fn record_render(&mut self, duration: Duration) {
           self.render_count += 1;
           self.render_time += duration;
       }
       
       pub fn record_version_check(&mut self, hit: bool) {
           self.version_checks += 1;
           if hit {
               self.version_hits += 1;
           }
       }
       
       pub fn cache_hit_rate(&self) -> f64 {
           if self.version_checks == 0 {
               return 0.0;
           }
           self.version_hits as f64 / self.version_checks as f64
       }
   }
   ```

2. **在视图中记录指标**
   ```rust
   cx.observe_global_in::<TodoStore>(window, move |this, window, cx| {
       let start = Instant::now();
       let store = cx.global::<TodoStore>();
       
       // 记录版本号检查
       let hit = this.cached_version == store.version();
       cx.global_mut::<PerformanceMetrics>()
           .record_version_check(hit);
       
       if hit {
           return;  // 缓存命中
       }
       
       // 更新视图...
       
       // 记录渲染时间
       let duration = start.elapsed();
       cx.global_mut::<PerformanceMetrics>()
           .record_render(duration);
   });
   ```

3. **添加性能报告**
   ```rust
   pub fn print_performance_report(cx: &App) {
       let metrics = cx.global::<PerformanceMetrics>();
       
       println!("=== Performance Report ===");
       println!("Render count: {}", metrics.render_count);
       println!("Average render time: {:?}", 
           metrics.render_time / metrics.render_count as u32);
       println!("Cache hit rate: {:.2}%", 
           metrics.cache_hit_rate() * 100.0);
       println!("Version checks: {}", metrics.version_checks);
       println!("Version hits: {}", metrics.version_hits);
   }
   ```

4. **添加基准测试**
   ```rust
   #[cfg(test)]
   mod benchmarks {
       use super::*;
       use criterion::{black_box, criterion_group, criterion_main, Criterion};
       
       fn bench_version_check(c: &mut Criterion) {
           let store = TodoStore::new();
           let cached_version = store.version();
           
           c.bench_function("version_check", |b| {
               b.iter(|| {
                   black_box(cached_version == store.version())
               })
           });
       }
       
       fn bench_batch_add(c: &mut Criterion) {
           c.bench_function("batch_add_100", |b| {
               b.iter(|| {
                   // 批量添加 100 个任务
                   let items = create_test_items(100);
                   batch_add_items(items, cx);
               })
           });
       }
       
       criterion_group!(benches, bench_version_check, bench_batch_add);
       criterion_main!(benches);
   }
   ```

---

### 阶段 5: 用户界面优化

**目标**: 改进用户界面，显示错误提示和加载状态

**进度**: ⏳ 待开始

**任务清单**:

1. **添加 Toast 提示组件**
   ```rust
   pub struct Toast {
       message: String,
       toast_type: ToastType,
       duration: Duration,
   }
   
   pub enum ToastType {
       Success,
       Error,
       Warning,
       Info,
   }
   
   impl Toast {
       pub fn show_error(message: impl Into<String>, cx: &mut App) {
           let toast = Toast {
               message: message.into(),
               toast_type: ToastType::Error,
               duration: Duration::from_secs(5),
           };
           cx.global_mut::<ToastManager>().show(toast);
       }
       
       pub fn show_success(message: impl Into<String>, cx: &mut App) {
           let toast = Toast {
               message: message.into(),
               toast_type: ToastType::Success,
               duration: Duration::from_secs(3),
           };
           cx.global_mut::<ToastManager>().show(toast);
       }
   }
   ```

2. **在错误处理中使用 Toast**
   ```rust
   match add_item(item, cx).await {
       Ok(_) => {
           Toast::show_success("任务添加成功", cx);
       }
       Err(e) => {
           let context = ErrorHandler::handle(e);
           Toast::show_error(context.user_message, cx);
       }
   }
   ```

3. **添加加载状态指示器**
   ```rust
   pub struct LoadingState {
       is_loading: bool,
       message: Option<String>,
   }
   
   // 在操作开始时显示加载状态
   cx.global_mut::<LoadingState>().start("正在添加任务...");
   
   // 操作完成后隐藏
   cx.global_mut::<LoadingState>().stop();
   ```

4. **添加确认对话框**
   ```rust
   pub fn show_confirmation_dialog(
       message: &str,
       on_confirm: impl Fn(&mut App) + 'static,
       cx: &mut App,
   ) {
       let dialog = ConfirmDialog::new(message)
           .on_confirm(on_confirm)
           .on_cancel(|| {
               // 取消操作
           });
       
       cx.show_dialog(dialog);
   }
   ```

---

## 📊 实施进度跟踪

### 总体进度

```
阶段 1: 错误处理应用    [████░░░░░░] 20%
阶段 2: 快捷键实现      [░░░░░░░░░░]  0%
阶段 3: 批量操作入口    [░░░░░░░░░░]  0%
阶段 4: 性能监控        [░░░░░░░░░░]  0%
阶段 5: 用户界面优化    [░░░░░░░░░░]  0%

总体进度: 4%
```

### 详细进度

| 阶段 | 任务 | 状态 | 优先级 |
|------|------|------|--------|
| 1 | item.rs 错误处理 | ✅ 完成 | 🔴 高 |
| 1 | project.rs 错误处理 | ⏳ 待开始 | 🔴 高 |
| 1 | label.rs 错误处理 | ⏳ 待开始 | 🔴 高 |
| 1 | section.rs 错误处理 | ⏳ 待开始 | 🔴 高 |
| 2 | 注册快捷键 | ⏳ 待开始 | 🔴 高 |
| 2 | 实现处理函数 | ⏳ 待开始 | 🔴 高 |
| 3 | 添加选择模式 | ⏳ 待开始 | 🟡 中 |
| 3 | 批量操作按钮 | ⏳ 待开始 | 🟡 中 |
| 4 | 性能指标收集 | ⏳ 待开始 | 🟡 中 |
| 4 | 基准测试 | ⏳ 待开始 | 🟡 中 |
| 5 | Toast 组件 | ⏳ 待开始 | 🟢 低 |
| 5 | 加载状态 | ⏳ 待开始 | 🟢 低 |

---

## 🎯 下一步行动

### 立即执行（本周）

1. ✅ 完成 `item.rs` 错误处理
2. ⏳ 完成其他 todo_actions 文件的错误处理
3. ⏳ 开始实现快捷键处理逻辑

### 短期目标（2 周内）

1. 完成所有错误处理应用
2. 实现核心快捷键功能
3. 添加基本的批量操作入口

### 中期目标（1 月内）

1. 完成所有快捷键实现
2. 完善批量操作功能
3. 添加性能监控
4. 改进用户界面

---

## 📝 注意事项

### 开发规范

1. **保持一致性**
   - 所有错误处理使用统一的模式
   - 所有快捷键遵循相同的命名规范
   - 所有日志使用结构化格式

2. **测试覆盖**
   - 每个新功能都要添加单元测试
   - 关键路径添加集成测试
   - 性能敏感代码添加基准测试

3. **文档更新**
   - 代码注释要清晰
   - API 文档要完整
   - 使用指南要及时更新

4. **性能考虑**
   - 避免不必要的克隆
   - 使用批量操作代替循环
   - 利用版本号缓存机制

### 常见问题

**Q: 如何在现有代码中应用错误处理？**

A: 按照以下步骤：
1. 添加输入验证
2. 使用 ErrorHandler 处理错误
3. 添加结构化日志
4. 显示用户友好的错误消息

**Q: 快捷键冲突怎么办？**

A: 
1. 检查系统快捷键
2. 遵循平台标准
3. 提供自定义选项（未来版本）

**Q: 如何测试性能优化效果？**

A:
1. 添加性能指标收集
2. 运行基准测试
3. 对比优化前后数据
4. 使用 flamegraph 分析

---

## 🔗 相关文档

- [优化进度](OPTIMIZATION_PROGRESS.md)
- [优化总结](OPTIMIZATION_SUMMARY.md)
- [错误处理指南](ERROR_HANDLING_GUIDE.md)
- [快捷键指南](SHORTCUTS_GUIDE.md)
- [批量操作指南](BATCH_OPERATIONS_GUIDE.md)

---

**最后更新**: 2026-02-19  
**负责人**: 开发团队  
**状态**: 🔄 进行中
