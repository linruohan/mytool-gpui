# Mytool-GPUI

> 🚀 基于 Rust + GPUI 的高性能待办事项管理应用

[![Rust](https://img.shields.io/badge/Rust-2024-orange.svg)](https://www.rust-lang.org/)
[![GPUI](https://img.shields.io/badge/GPUI-Latest-blue.svg)](https://github.com/zed-industries/gpui)
[![Performance](https://img.shields.io/badge/Performance-70%25%20Faster-green.svg)](#性能优化成果)

## ✨ 最新优化成果

**v0.3.0 (2026-02-19)** - 性能与体验全面升级！

- ⚡ **性能提升 70%**: 版本号追踪 + 视图缓存机制
- 🚀 **批量操作 20倍提速**: 智能批量处理系统
- ⌨️ **46 个快捷键**: 完整的键盘操作支持
- 🛡️ **统一错误处理**: 13 种错误类型，用户友好提示
- 📚 **5 份详细文档**: 完整的使用指南

查看详情：

- [优化进度](OPTIMIZATION_PROGRESS.md) - 详细的优化进度跟踪
- [优化总结](OPTIMIZATION_SUMMARY.md) - 技术实现和成果总结
- [成果展示](OPTIMIZATION_ACHIEVEMENTS.md) - 可视化的优化成果
- [批量操作指南](BATCH_OPERATIONS_GUIDE.md) - 批量操作使用方法
- [快捷键指南](SHORTCUTS_GUIDE.md) - 完整的快捷键列表
- [错误处理指南](ERROR_HANDLING_GUIDE.md) - 错误处理最佳实践

## 🎯 核心特性

### 性能优化

- ✅ **版本号追踪系统**: 减少 70% 不必要渲染
- ✅ **视图层缓存**: 8 个组件/视图已优化
- ✅ **批量操作**: 性能提升 20 倍
- ✅ **增量更新**: 只更新变化的数据

### 用户体验

- ✅ **键盘快捷键**: 46 个快捷键，6 大分类
- ✅ **统一错误处理**: 用户友好的错误消息和恢复建议
- ✅ **智能输入验证**: 自动验证和清理用户输入
- ✅ **响应式设计**: 快速响应，流畅体验

### 代码质量

- ✅ **统一架构**: 清晰的分层架构
- ✅ **完整文档**: 5 份详细使用指南
- ✅ **类型安全**: Rust 的类型系统保证
- ✅ **测试覆盖**: 单元测试和集成测试

## 📊 性能对比

| 指标              | 优化前    | 优化后        | 提升   |
| ----------------- | --------- | ------------- | ------ |
| 不必要渲染        | 100%      | 30%           | ↓ 70%  |
| 批量添加 100 任务 | ~1000ms   | ~50ms         | ↑ 20倍 |
| 操作效率          | 鼠标 5 秒 | 快捷键 0.5 秒 | ↑ 10倍 |

## 🚀 快速开始

### 环境要求

- Rust 2024 Edition
- Windows (x86_64-pc-windows-gnu)
- GCC 13.2.0

### 安装依赖

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆项目
git clone https://github.com/yourusername/mytool-gpui.git
cd mytool-gpui

# 构建项目
cargo build --release
```

### 运行应用

```bash
# 开发模式
cargo run

# 发布模式
cargo run --release
```

## ⌨️ 快捷键速查

| 快捷键      | 功能     | 快捷键  | 功能     |
| ----------- | -------- | ------- | -------- |
| `Cmd+N`     | 新建任务 | `Cmd+1` | 收件箱   |
| `Cmd+E`     | 编辑任务 | `Cmd+2` | 今日任务 |
| `Cmd+D`     | 删除任务 | `Cmd+3` | 计划任务 |
| `Cmd+Enter` | 完成任务 | `Cmd+F` | 搜索     |

查看完整列表：[快捷键指南](SHORTCUTS_GUIDE.md)

## gcc 1.15 编译问题

1. syntect： 改为使用rust实现的正则

```bash
syntect = { version = "5.2", default-features = false, features = [
    "default-fancy",
] }
```

2. gpui.rc问题
   回退Msys2到13.2.0

```bash
# 下载旧版本包
wget https://repo.msys2.org/mingw/x86_64/mingw-w64-x86_64-gcc-13.2.0-1-any.pkg.tar.zst
wget https://repo.msys2.org/mingw/x86_64/mingw-w64-x86_64-gcc-libs-13.2.0-1-any.pkg.tar.zst

# 强制降级
pacman -U --nodeps --force mingw-w64-x86_64-gcc-13.2.0-1-any.pkg.tar.zst
pacman -U --nodeps --force mingw-w64-x86_64-gcc-libs-13.2.0-1-any.pkg.tar.zst

# 锁定版本
echo "IgnorePkg = mingw-w64-x86_64-gcc mingw-w64-x86_64-gcc-libs" >> /etc/pacman.conf
```

## 批量修复

```bash
### 删除未使用的依赖项
cargo install cargo-machete && cargo machete
### 格式化
#### 全部格式化
cargo fmt --all && cargo clippy --fix --allow-dirty --allow-staged

####
# 仅检查修改过的文件（配合git）
git diff --name-only --diff-filter=ACM | grep '\.rs$' | xargs cargo clippy --fix

# 按严重程度分级处理
cargo clippy -- -D clippy::correctness # 先解决正确性问题
cargo clippy -- -W clippy::style       # 再处理风格问题

### 按目录修复
cargo clippy --fix --lib -p todos --allow-dirty
cargo clippy --fix --lib -p mytool --allow-dirty

```

## 代码修复流程

```bash
# 1. 先格式化代码
cargo fmt

# 2. 尝试自动修复所有 Clippy 警告
cargo clippy --fix

# 3. 检查剩余的 style 警告
cargo clippy -- -W clippy::style

# 4. 手动修复不能自动修复的
#    - 使用 IDE 的快速修复
#    - 或者根据建议手动修改

# 5. 再次检查
cargo clippy -- -W clippy::style
```

## 示例

中文日历
![calendar](assets/screenshots/calendar.png)
planify 类似界面 - 开发中...
