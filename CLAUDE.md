# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MyTool-GPUI is a Rust-based desktop todo/task management application built with the GPUI (GPU-accelerated UI) framework from Zed Industries. The project uses a layered architecture with Sea-ORM for SQLite database access and supports multiple languages (English, Chinese, Spanish, French, German, Italian).

**Toolchain:** `x86_64-pc-windows-gnu` (MSYS2 environment)

## Development Commands

### Build
```bash
cargo build                    # Debug build
cargo build --release          # Release build (opt-level 3, LTO enabled)
cargo build -p mytool          # Build main app crate only
cargo build --profile fast     # Ultra-fast build for quick verification
```

### Run
```bash
cargo run                      # Run debug build
cargo run --release            # Run release build
```

### Test
```bash
cargo test                     # Run all tests
cargo test -p todos            # Run tests for todos crate only
cargo test test_name           # Run specific test by name
cargo test -- --nocapture      # Run tests with stdout output
```

### Format & Lint
```bash
# Format and auto-fix in one command
cargo fmt --all && cargo clippy --fix --allow-dirty --allow-staged

# Check for unused dependencies
cargo install cargo-machete && cargo machete

# Fix by severity level
cargo clippy -- -D clippy::correctness    # Fix correctness issues first
cargo clippy -- -W clippy::style          # Then handle style warnings

# Fix specific crate
cargo clippy --fix --lib -p todos --allow-dirty
cargo clippy --fix --lib -p mytool --allow-dirty
```

### Sea-ORM Entity Generation
```bash
# Generate entities from SQLite database
sea-orm-cli generate entity --with-serde both --model-extra-attributes 'serde(rename_all="camelCase")' --date-time-crate chrono -o ./src/entity --database-url "sqlite://db.sqlite?mode=rwc"
```

## Workspace Structure

```
crates/
├── mytool/         # Main GUI application (GPUI-based)
├── todos/          # Core task management library (data layer)
└── gconfig/        # Global configuration management
test/               # Test utilities and benchmarks
```

## Architecture

### Layered Architecture

```
┌─────────────────────────────────────────┐
│   mytool (GUI Layer)                    │
│   - Stories, Views, Components          │
│   - Global State (DBState, TodoStore)   │
├─────────────────────────────────────────┤
│   Service Layer (todos/services/)      │
│   - ItemService, LabelService, etc.     │
│   - QueryService, EventBus, Store       │
├─────────────────────────────────────────┤
│   Repository Layer (todos/repositories/)│
│   - Data access patterns                │
├─────────────────────────────────────────┤
│   Entity Layer (todos/entity/)          │
│   - Sea-ORM generated models            │
├─────────────────────────────────────────┤
│   SQLite Database                       │
└─────────────────────────────────────────┘
```

### Key Entities (todos/entity/)
- **ItemModel** - Tasks with content, due dates, priority, labels
- **ProjectModel** - Project containers
- **SectionModel** - Sections within projects
- **LabelModel** - Tags/labels for items
- **ReminderModel** - Reminder notifications
- **AttachmentModel** - File attachments
- **SourceModel** - Data sources
- **QueueModel** - Task queue
- **OEventModel** - Operations/events
- **CurTempIdModel** - Temporary ID tracking

### Service Layer (todos/services/)
Core services coordinate business logic:
- `ItemService` - CRUD operations for items
- `LabelService`, `ProjectService`, `SectionService`, `ReminderService`
- `QueryService` - Batch query operations with concurrency control
- `EventBus` - Event-driven communication between components
- `Store` - Centralized state management
- `ServiceManager` - Coordinates all services

### Global State Management (mytool/core/state/)
GPUI uses a global state pattern:
- `DBState` - Global database connection
- `TodoStore` - Single source of truth for all data
- `TodoEventBus` - Event distribution
- `QueryCache` - LRU caching for performance
- `BatchOperations` - Batch operation queue
- `ErrorNotifier` - Error notification handling
- `ObserverRegistry` - State observers
- `DirtyFlags` - Change tracking
- `PendingTasksState` - Async task tracking

State initialization happens in `state_init(cx: &mut App, db: DatabaseConnection)`.

### UI Components (mytool/ui/)
- **Stories** (`stories/`) - Storybook-style demo views (WelcomeStory, CalendarStory, TodoStory, ListStory)
- **Views** (`views/`) - Main views including board views (Inbox, Today, Scheduled, Pin, Completed, Labels)
- **Components** (`components/`) - Reusable UI components (buttons, popovers, dialogs, etc.)
- **Widgets** (`widgets/`) - Custom widgets

## Build Profile Configuration

- **dev profile:** `opt-level = 0`, `debug = 0`, incremental builds, codegen-units = 16
- **fast profile:** `opt-level = 0`, codegen-units = 256, no incremental (for sccache)
- **release profile:** `opt-level = 3`, LTO enabled, stripped binaries
- **High-performance packages:** `resvg`, `rustybuzz`, `taffy`, `ttf-parser`, `gpui` use `opt-level = 3` even in dev profile

## Code Style Configuration

Defined in `rustfmt.toml`:
- `max_width = 100`
- `imports_granularity = "Crate"`
- `group_imports = "StdExternalCrate"`
- `use_field_init_shorthand = true`
- `format_strings = true`
- `wrap_comments = true`

## GCC 1.15 Compilation Issues

If encountering compilation errors with newer GCC versions:

1. **syntect:** Use `default-fancy` feature instead of default
```toml
syntect = { version = "5.2", default-features = false, features = ["default-fancy"] }
```

2. **Downgrade GCC to 13.2.0 in MSYS2:**
```bash
wget https://repo.msys2.org/mingw/x86_64/mingw-w64-x86_64-gcc-13.2.0-1-any.pkg.tar.zst
wget https://repo.msys2.org/mingw/x86_64/mingw-w64-x86_64-gcc-libs-13.2.0-1-any.pkg.tar.zst
pacman -U --nodeps --force mingw-w64-x86_64-gcc-13.2.0-1-any.pkg.tar.zst
pacman -U --nodeps --force mingw-w64-x86_64-gcc-libs-13.2.0-1-any.pkg.tar.zst
echo "IgnorePkg = mingw-w64-x86_64-gcc mingw-w64-x86_64-gcc-libs" >> /etc/pacman.conf
```

## Internationalization

Supported languages: English (en), Chinese Simplified (zh-CN), Chinese Traditional (zh-HK), Spanish (es), French (fr), German (de), Italian (it).

Locale files: `crates/mytool/locales/ui.yml`

## Themes

Theme files are in `themes/` directory as JSON files. Default theme is `themes/default.json`. Over 20 themes available (catppuccin, tokyonight, gruvbox, ayu, solarized, etc.).

## Plugin System

Located in `crates/mytool/src/plugins/`:
- `Plugin` - Base trait for plugins
- `PluginRegistry` - Plugin discovery and management
- `PluginConfig` - Plugin configuration structure

## External Dependencies

- **GPUI:** `git = "https://github.com/zed-industries/zed"`
- **GPUI Component:** `git = "https://github.com/linruohan/gpui-component.git"`
- **Sea-ORM:** v1.1.19 with SQLite, chrono, JSON support
- **Tokio:** Full features for async runtime
- **rust-i18n:** v3.1.5 for i18n
