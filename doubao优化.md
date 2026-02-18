# Mytool-GPUI 优化方案

## 📋 概述

本文档基于对项目架构的全面分析，提出项目优化方案。按照优先级从高到低逐步实施。

---

## 🎯 优化优先级总览

| 优先级 | 方案 | 预期效果 | 工作量 | 风险 | 状态 |
|--------|------|----------|--------|------|------|
| 🔴 高 | 完全统一状态管理 | 消除状态不一致，代码更简洁 | 中 | 低 | ⏳ 待开始 |
| 🟡 中 | 优化数据过滤性能 | 查询速度提升 | 中 | 低 | ⏳ 待开始 |
| 🟡 中 | 完善增量更新应用 | 性能更稳定 | 低 | 低 | ⏳ 待开始 |
| 🟡 中 | 统一视图渲染组件 | 减少代码重复 | 中 | 中 | ⏳ 待开始 |
| 🟢 低 | 代码清理和文档 | 可维护性提升 | 低 | 低 | ⏳ 待开始 |

---

## 🏗️ 方案一：完全统一状态管理（高优先级）

### 问题分析

当前存在两套状态管理：
1. **TodoStore**：新架构，统一管理 Item 数据
2. **分散状态**：ItemState、ProjectState、LabelState、SectionState

导致的问题：
- 需要同步两套状态，容易不一致
- 代码冗余
- 维护成本高

### 优化目标

把所有状态都整合到 TodoStore 中，成为唯一数据源。

### 实施步骤

#### 1. 扩展 TodoStore

在 `crates/mytool/src/todo_state/todo_store.rs` 中添加：
- Project 相关的字段和方法
- Label 相关的字段和方法
- Section 相关的字段和方法

#### 2. 更新 state_init

在 `crates/mytool/src/todo_state/mod.rs` 中：
- 简化状态初始化
- 移除分散状态的初始化
- 只保留 TodoStore

#### 3. 更新所有视图组件

检查并更新所有使用分散状态的组件，改为使用 TodoStore。

#### 4. 删除分散状态文件

删除以下文件：
- `crates/mytool/src/todo_state/item.rs`
- `crates/mytool/src/todo_state/project.rs`
- `crates/mytool/src/todo_state/label.rs`
- `crates/mytool/src/todo_state/section.rs`

### 预期效果

- 代码减少约 300-500 行
- 消除状态不一致风险
- 维护更简单

---

## ⚡ 方案二：优化数据过滤性能（中优先级）

### 问题分析

TodoStore 的过滤方法每次都要遍历所有数据：
```rust
pub fn inbox_items(&self) -> Vec<Arc<ItemModel>> {
    self.all_items.iter().filter(|item| ...).cloned().collect()
}
```

### 优化思路

建立索引结构，实现快速查找：

```rust
use std::collections::{HashMap, HashSet};

pub struct TodoStore {
    all_items: Vec<Arc<ItemModel>>,
    projects: Vec<Arc<ProjectModel>>,
    labels: Vec<Arc<LabelModel>>,
    sections: Vec<Arc<SectionModel>>,
    
    // 索引结构
    project_index: HashMap<String, Vec<Arc<ItemModel>>>,
    section_index: HashMap<String, Vec<Arc<ItemModel>>>,
    checked_set: HashSet<String>,
    pinned_set: HashSet<String>,
}
```

### 实施步骤

1. 添加索引字段到 TodoStore
2. 实现索引更新逻辑（在增量更新时同步更新索引）
3. 优化过滤方法，使用索引

### 预期效果

- 查询速度提升 50%+
- 大数据量时性能更稳定

---

## 🔄 方案三：完善增量更新应用（中优先级）

### 问题分析

虽然 TodoStore 有增量更新方法，但可能不是所有地方都在使用。

### 检查清单

- [ ] 所有 CRUD 操作都使用增量更新
- [ ] 删除不需要的全量刷新代码
- [ ] 添加变更通知机制
- [ ] 验证增量更新的正确性

### 实施步骤

1. 检查所有 todo_actions 中的操作
2. 确保都使用增量更新
3. 清理 store_actions 中不需要的全量刷新
4. 添加测试验证增量更新

---

## 🎨 方案四：统一视图渲染组件（中优先级）

### 问题分析

board_renderer.rs 已经写好了，但可能还没有完全应用到所有 Board。

### 实施步骤

1. 检查所有 Board 组件
2. 确保都使用 board_renderer 中的统一方法
3. 提取更多通用组件

---

## 🧹 方案五：代码清理和文档优化（低优先级）

### 清理内容

1. 清理未使用的代码
   - 检查是否有不再使用的模块
   - 删除注释掉的代码

2. 添加单元测试
   - 为 TodoStore 的过滤方法添加测试
   - 为增量更新逻辑添加测试

3. 完善文档
   - 更新架构图
   - 添加使用示例

---

## 📅 实施计划

### 第一阶段（1-2 天）
- [ ] 完全统一状态管理

### 第二阶段（2-3 天）
- [ ] 优化数据过滤性能
- [ ] 完善增量更新应用

### 第三阶段（按需）
- [ ] 统一视图渲染组件
- [ ] 代码清理和文档

---

## 📝 变更日志

| 日期 | 版本 | 说明 |
|------|------|------|
| 2026-02-18 | 1.0 | 初始版本 |
