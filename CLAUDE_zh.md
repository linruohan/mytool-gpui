# CLAUDE.md

本文档为 Claude Code (claude.ai/code) 在此代码仓库中工作提供指导。

## 项目概述

MyTool-GPUI 是一个基于 Rust 和 GPUI（GPU 加速 UI 框架，来自 Zed Industries）开发的桌面待办/任务管理应用。项目采用分层架构，使用 Sea-ORM 进行 SQLite 数据库访问，支持多语言（英语、中文、西班牙语、法语、德语、意大利语）。

**工具链：** `x86_64-pc-windows-gnu` (MSYS2 环境)

## 开发命令

### 构建
```bash
cargo build                    # 调试构建
cargo build --release          # 发布构建（opt-level 3，启用 LTO）
cargo build -p mytool          # 仅构建主应用 crate
cargo build --profile fast     # 超快速构建，用于快速验证
```

### 运行
```bash
cargo run                      # 运行调试版本
cargo run --release            # 运行发布版本
```

### 测试
```bash
cargo test                     # 运行所有测试
cargo test -p todos            # 仅运行 todos crate 的测试
cargo test test_name           # 运行指定名称的测试
cargo test -- --nocapture      # 运行测试并输出 stdout
```

### 格式化与检查
```bash
# 格式化并自动修复
cargo fmt --all && cargo clippy --fix --allow-dirty --allow-staged

# 检查未使用的依赖
cargo install cargo-machete && cargo machete

# 按严重程度修复
cargo clippy -- -D clippy::correctness    # 先修复正确性问题
cargo clippy -- -W clippy::style          # 再处理风格警告

# 修复特定 crate
cargo clippy --fix --lib -p todos --allow-dirty
cargo clippy --fix --lib -p mytool --allow-dirty
```

### Sea-ORM 实体生成
```bash
# 从 SQLite 数据库生成实体
sea-orm-cli generate entity --with-serde both --model-extra-attributes 'serde(rename_all="camelCase")' --date-time-crate chrono -o ./src/entity --database-url "sqlite://db.sqlite?mode=rwc"
```

## Workspace 结构

```
crates/
├── mytool/         # 主 GUI 应用（基于 GPUI）
├── todos/          # 核心任务管理库（数据层）
└── gconfig/        # 全局配置管理
test/               # 测试工具和基准测试
```

## 架构

### 分层架构

```
┌─────────────────────────────────────────┐
│   mytool (GUI 层)                        │
│   - Boards / Views / Components         │
│   - Globals: DBState, TodoStore, Cache  │
│   - 写路径：optimistic actions           │
├─────────────────────────────────────────┤
│   服务层 (todos/services/)              │
│   - Item/Label/Project/Section/...      │
│   - Store（GUI/冷加载薄门面）             │
├─────────────────────────────────────────┤
│   仓库层 (todos/repositories/)          │
│   - Item/Label/Section (+ BaseRepository)│
├─────────────────────────────────────────┤
│   实体层 (todos/entity/)                │
│   - Sea-ORM 生成的模型                   │
├─────────────────────────────────────────┤
│   SQLite 数据库 (WAL)                   │
└─────────────────────────────────────────┘
```

UI 刷新依赖 `TodoStore` + `ChangeMask` + `QueryCache`（无领域 EventBus）。

### 核心实体 (todos/entity/)
活跃领域模型：
- **ItemModel** - 任务，包含内容、到期日期、优先级、标签
- **ProjectModel** - 项目容器
- **SectionModel** - 项目内的分区
- **LabelModel** - 任务的标签/标记
- **ReminderModel** - 提醒通知
- **AttachmentModel** - 文件附件

Schema 残留（已生成、无服务路径）：`SourceModel`、`QueueModel`、`OEventModel`、`CurTempIdModel`。

### 服务层 (todos/services/)
- `ItemService`、`LabelService`、`ProjectService`、`SectionService`、`ReminderService`、`AttachmentService`
- `Store` - GUI 冷加载与 `state_service` 使用的薄透传门面

### 全局状态管理 (mytool/core/state/)
在 `state_init(cx, db)` 中初始化：
- `DBState` - 数据库连接 + 异步 `todos::Store` 初始化
- `TodoStore` - 内存唯一数据源 + 索引 + `ChangeMask`
- `QueryCache` - 看板过滤结果的 Arc 共享缓存
- `ErrorNotifier` / `PendingTasksState` / `SaveResults` - 错误与保存状态跟踪

### UI 组件 (mytool/ui/)
- **Views** (`views/boards/`) - Inbox、Today、Scheduled、Pin、Completed、Labels（共享 `board_base` / `board_common` / `board_renderer`）
- **Components** (`components/`) - `item_row`（懒加载 `ItemInfo`）、弹窗、对话框
- **Stories** (`stories/`) - Storybook 风格演示视图
- **Widgets** (`widgets/`) - 自定义组件

## 构建配置

- **dev profile:** `opt-level = 0`, `debug = 0`, 增量构建，codegen-units = 16
- **fast profile:** `opt-level = 0`, codegen-units = 256, 非增量构建（配合 sccache）
- **release profile:** `opt-level = 3`, 启用 LTO, 剥离符号
- **高性能包：** `resvg`, `rustybuzz`, `taffy`, `ttf-parser`, `gpui` 在 dev profile 中也使用 `opt-level = 3`

## 代码风格配置

定义于 `rustfmt.toml`：
- `max_width = 100`
- `imports_granularity = "Crate"`
- `group_imports = "StdExternalCrate"`
- `use_field_init_shorthand = true`
- `format_strings = true`
- `wrap_comments = true`

## GCC 1.15 编译问题

使用较新 GCC 版本遇到编译错误时：

1. **syntect:** 使用 `default-fancy` 特性替代默认特性
```toml
syntect = { version = "5.2", default-features = false, features = ["default-fancy"] }
```

2. **在 MSYS2 中降级 GCC 到 13.2.0：**
```bash
wget https://repo.msys2.org/mingw/x86_64/mingw-w64-x86_64-gcc-13.2.0-1-any.pkg.tar.zst
wget https://repo.msys2.org/mingw/x86_64/mingw-w64-x86_64-gcc-libs-13.2.0-1-any.pkg.tar.zst
pacman -U --nodeps --force mingw-w64-x86_64-gcc-13.2.0-1-any.pkg.tar.zst
pacman -U --nodeps --force mingw-w64-x86_64-gcc-libs-13.2.0-1-any.pkg.tar.zst
echo "IgnorePkg = mingw-w64-x86_64-gcc mingw-w64-x86_64-gcc-libs" >> /etc/pacman.conf
```

## 国际化

支持语言：英语 (en)、简体中文 (zh-CN)、繁体中文 (zh-HK)、西班牙语 (es)、法语 (fr)、德语 (de)、意大利语 (it)。

语言文件：`crates/mytool/locales/ui.yml`

## 主题

主题文件位于 `themes/` 目录，格式为 JSON。默认主题是 `themes/default.json`。提供超过 20 个主题（catppuccin、tokyonight、gruvbox、ayu、solarized 等）。

## 外部依赖

- **GPUI:** `git = "https://github.com/zed-industries/zed"`
- **GPUI Component:** `git = "https://github.com/linruohan/gpui-component.git"`
- **Sea-ORM:** v1.1.19，支持 SQLite、chrono、JSON
- **Tokio:** 完整特性的异步运行时
- **rust-i18n:** v3.1.5 用于国际化
