# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MyTool-GPUI is a Rust-based desktop todo/task management application built with the GPUI (GPU-accelerated UI) framework from Zed Industries. The project uses a layered architecture with Sea-ORM for SQLite database access and supports multiple languages (English, Chinese, Spanish, French, German).

**Toolchain:** `x86_64-pc-windows-gnu` (MSYS2 environment)

## Development Commands

### Build
```bash
cargo build                    # Debug build
cargo build --release          # Release build (opt-level 3, LTO enabled)
cargo build -p mytool         # Build main app crate only
```

### Test
```bash
cargo test                     # Run all tests
```

### Format & Lint (Standard Workflow)
```bash
# Format and auto-fix in one command
cargo fmt --all && cargo clippy --fix --allow-dirty --allow-staged

# Check for unused dependencies
cargo install cargo-machete && cargo machete
```

### Clippy - Fix by Priority
```bash
# Fix correctness issues first
cargo clippy -- -D clippy::correctness

# Then handle style warnings
cargo clippy -- -W clippy::style

# Fix specific crate
cargo clippy --fix --lib -p todos --allow-dirty
cargo clippy --fix --lib -p mytool --allow-dirty
```

### Sea-ORM Entity Generation (todos crate)
```bash
# Generate entities from SQLite database
sea-orm-cli generate entity --with-serde both --model-extra-attributes 'serde(rename_all="camelCase")' --date-time-crate chrono -o ./src/entity --database-url "sqlite://db.sqlite?mode=rwc"
```

## Workspace Structure

The project is a Cargo workspace with 3 member crates:

```
crates/
├── mytool/         # Main GUI application (GPUI-based)
├── todos/          # Core task management library (data layer)
└── gconfig/        # Global configuration management
```

## Architecture

### Layered Architecture (todos crate)

```
┌─────────────────────────────────────────┐
│   mytool (GUI Layer)                    │
│   - Stories, Views, Components          │
│   - Global State (ItemState, etc.)     │
├─────────────────────────────────────────┤
│   Service Layer (todos/services/)      │
│   - ItemService, LabelService, etc.     │
│   - QueryService, EventBus              │
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
- `CacheManager` - LRU caching for performance
- `ServiceManager` - Coordinates all services

### Global State Management (mytool/todo_state/)
GPUI uses a global state pattern. Each entity type has a corresponding state:
- `DBState` - Global database connection
- `ItemState`, `ProjectState`, `LabelState`, `SectionState` - Entity-specific states
- `InboxItemState`, `TodayItemState`, `ScheduledItemState`, `PinnedItemState`, `CompleteItemState` - Filtered item states

State initialization happens in `state_init(cx: &mut App)`.

### UI Components (mytool/)
- **Stories** (`stories/`) - Storybook-style demo views (WelcomeStory, CalendarStory, TodoStory, ListStory)
- **Views** (`views/`) - Main views including board views (Inbox, Today, Scheduled, Pin, Completed, Labels)
- **Components** (`components/`) - Reusable UI components (buttons, popovers, etc.)
- **Widgets** (`widgets/`) - Custom widgets

## Build Profile Configuration

- **dev profile:** `opt-level = 2`, `debug = 0`, incremental builds enabled
- **release profile:** `opt-level = 3`, LTO enabled, stripped binaries
- **High-performance packages:** `resvg`, `rustybuzz`, `taffy`, `ttf-parser` use `opt-level = 3` even in dev profile

## Code Style Configuration

Defined in `rustfmt.toml`:
- `max_width = 100`
- `imports_granularity = "Crate"`
- `group_imports = "StdExternalCrate"`
- `use_field_init_shorthand = true`

## Known Issues

### GCC 1.15 Compilation Issues
If encountering compilation errors with newer GCC versions:
1. Downgrade GCC to 13.2.0 in MSYS2
2. Or disable `syntect` default features and use `default-fancy` instead
3. Lock the GCC version in `/etc/pacman.conf` with `IgnorePkg = mingw-w64-x86_64-gcc mingw-w64-x86_64-gcc-libs`

## Internationalization

Supported languages: English (en), Chinese Simplified (zh-CN), Chinese Traditional (zh-HK), Spanish (es), French (fr), German (de).

Locale files: `crates/mytool/locales/ui.yml`

## Themes

Theme files are in `themes/` directory as JSON files. Default theme is `themes/default.json`. Over 20 themes available (catppuccin, tokyonight, gruvbox, etc.).

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
