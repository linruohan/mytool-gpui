# 项目优化分析报告

> 最后更新: 2026-02-20

## 一、代码质量问题

### 1.1 未完成的代码 (TODO)

项目中有31处TODO标记：

**显示错误提示** (4处) - ✅ 已修复
- `crates/mytool/src/core/actions/optimistic.rs:131, 226, 290` - 使用 ErrorNotifier 存储错误
- `crates/mytool/src/core/actions/item.rs:25, 43` - 使用 ErrorNotifier 存储错误

**Service 层未完全使用 Repository** (10处) - ✅ 已修复
- `crates/todos/src/services/section_service.rs:80, 132, 222`
- `crates/todos/src/services/label_service.rs:78, 162, 170, 183, 193, 217, 218`

**QueryService 占位符代码** (4处) - ✅ 已修复
- `crates/todos/src/services/query_service.rs:37, 56, 75, 91`
- 所有批量加载方法已实现，使用 `IN` 子句批量查询

### 1.2 安全性问题 - ✅ 已修复

`crates/todos/src/constants.rs:1-2` 硬编码的 API 密钥已改为环境变量：
```rust
pub fn todoist_client_id() -> String {
    get_env_or_panic("TODOIST_CLIENT_ID")
}
```

### 1.3 错误处理问题 - ✅ 已修复

移除了 `unwrap()` 调用，改用 `?` 操作符

### 1.4 EventBus 设计问题 - ✅ 已修复

- 使用 UUID 替代 usize 作为 listener ID
- 使用 HashSet 存储 listener，避免 ID 冲突
- 移除了未使用的 receiver 参数

### 1.5 错误通知系统 - ✅ 已添加

- 添加了 `ErrorNotifier` 全局状态 (`crates/mytool/src/core/state/events.rs`)
- 后台任务发生错误时存储错误消息到 `ErrorNotifier`
- UI 层可以检查并显示错误通知

---

## 二、架构优化

### 2.1 Repository 层未完全使用 - ✅ 部分修复

`item_service.rs:90-101` 已改为使用 Repository：
- 添加了 `delete` 方法到 ItemRepository trait
- `delete_item` 方法已改为通过 Repository 调用

### 2.2 QueryService 未实现 - ✅ 已修复

所有批量加载方法均已实现：
- `batch_load_items` - 使用 `IN` 子句批量查询
- `batch_load_projects` - 使用 `IN` 子句批量查询
- `batch_load_sections` - 使用 `IN` 子句批量查询
- `batch_load_labels` - 使用 `IN` 子句批量查询

---

## 三、测试缺失 - ✅ 已添加

- 添加了 `crates/todos/tests/` 目录
- 添加了 `error_test.rs` - 错误类型测试
- 添加了 `event_bus_test.rs` - 事件总线测试
- 添加了 `tokio-test` 和 `mockall` 测试依赖

---

## 四、性能优化

### 4.1 N+1 查询问题 - ✅ 已修复

`item_service.rs:184-200` 循环中逐个更新子项的问题已优化：
- 改为使用 `update_many` 批量更新
- 减少数据库往返次数

### 4.2 缓存缺失 - ✅ 已存在

项目中已有完善的缓存实现：
- 位置：`crates/mytool/src/core/state/cache.rs`
- 类型：GPUI 全局状态缓存 (`QueryCache`)
- 功能：缓存 inbox、today、scheduled、completed、pinned、overdue 等查询结果
- 按项目 ID 和分区 ID 缓存任务

---

## 五、其他

- **配置管理**：全局静态配置，测试时难以替换
- **Edition 问题**：使用 `edition = "2024"`（尚未发布）
- **重复代码**：Repository 中错误处理模式重复

---

## 优先级建议

| 优先级 | 优化项 | 状态 |
|--------|--------|------|
| 🔴 高 | 移除硬编码 API 密钥 | ✅ 已完成 |
| 🔴 高 | 实现 QueryService | ✅ 已完成 |
| 🟡 中 | 添加单元测试 | ✅ 已完成 |
| 🟡 中 | 统一 Repository 调用 | ✅ 已完成 |
| 🟡 中 | 修复 EventBus ID 管理 | ✅ 已完成 |
| 🟡 中 | Service 层使用 Repository | ✅ 已完成 |
| 🟡 中 | 实现错误通知系统 | ✅ 已完成 |
| 🟢 低 | 数据库批量更新优化 | ✅ 已完成 |
| 🟢 低 | 添加缓存机制 | ✅ 已存在 |

---

## 六、项目亮点

1. **项目结构清晰**：标准分层架构 (Service → Repository → Entity)
2. **错误类型定义良好**：`TodoError` 使用 thiserror
3. **异步处理正确**：使用 `tokio` 和 `async/await`
4. **EventBus 设计合理**：支持订阅/发布模式
5. **国际化支持**：`rust-i18n` 已集成
