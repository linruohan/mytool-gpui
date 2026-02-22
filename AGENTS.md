# AGENTS.md - Agent Guidelines for MyTool-GPUI

This file provides guidance for AI coding agents working in this repository.

## Project Overview

MyTool-GPUI is a Rust-based desktop todo/task management application built with GPUI framework from Zed Industries. Uses Sea-ORM with SQLite for data persistence.

**Toolchain:** `x86_64-pc-windows-msvc` (MSYS2 environment)
**Workspace:** 3 crates - `mytool` (GUI), `todos` (core library), `gconfig` (config)

---

## Build Commands

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Build specific crate
cargo build -p mytool
```

---

## Lint & Format Commands

```bash
# Format code
cargo fmt --all

# Run clippy with auto-fix
cargo clippy --fix --allow-dirty --allow-staged

# Run clippy with specific priorities
cargo clippy -- -D clippy::correctness    # Fix correctness first
cargo clippy -- -W clippy::style           # Then style warnings

# Fix specific crate
cargo clippy --fix --lib -p todos --allow-dirty
cargo clippy --fix --lib -p mytool --allow-dirty
```

---

## Test Commands

```bash
# Run all tests
cargo test

# Run single test by name
cargo test test_name_here

# Run tests in specific crate
cargo test -p todos

# Run with output
cargo test -- --nocapture
```

---

## Code Style Guidelines

### Formatting (rustfmt.toml)

- **Max line width:** 100 characters
- **Tab size:** 4 spaces
- **Import granularity:** Crate-level (`use crate::module::...`)
- **Group imports:** StdExternalCrate
- **Use field init shorthand:** Yes
- **Use try shorthand:** Yes
- **Format strings:** Yes

### Clippy Configuration

- **Allowed:** Private module inception
- **Disallowed methods:** Use `smol::process::Command` instead of `std::process::Command`
- **Disallowed:** `serde_json::from_reader` - use `from_slice` instead

### Naming Conventions

- **Snake_case:** Variables, functions, modules
- **PascalCase:** Types, traits, enums
- **SCREAMING_SNAKE_CASE:** Constants
- **Prefix `_`:** Unused variables (e.g., `let _unused = ...`)

### Error Handling

- Use `anyhow` for application-level error handling
- Use `thiserror` for library error types
- Avoid `unwrap()` in production code
- Use `?` operator for propagating errors

### Imports

```rust
// Group order: std -> external -> crate
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::services::ItemService;
```

### Async & Concurrency

- Use `tokio` for async runtime
- Use `smol::process::Command` for spawning processes (not `std::process::Command`)
- Follow async/await best practices

### Sea-ORM Entity Generation

```bash
sea-orm-cli generate entity --with-serde both \
  --model-extra-attributes 'serde(rename_all="camelCase")' \
  --date-time-crate chrono -o ./src/entity \
  --database-url "sqlite://db.sqlite?mode=rwc"
```

---

## Architecture

```
mytool (GUI Layer) -> Services -> Repositories -> Sea-ORM Entities -> SQLite
```

### Key Layers

- **Entity:** Sea-ORM models in `todos/entity/`
- **Repository:** Data access in `todos/repositories/`
- **Service:** Business logic in `todos/services/`
- **GUI:** GPUI views/components in `mytool/src/`

### Global State (GPUI)

- `DBState` - Database connection
- `ItemState`, `ProjectState`, `LabelState`, `SectionState` - Entity states
- Filtered states: `InboxItemState`, `TodayItemState`, `ScheduledItemState`, etc.

---

## Internationalization

- Uses `rust-i18n` v3.1.5
- Locale files: `crates/mytool/locales/ui.yml`
- Supported: English, Chinese (Simplified/Traditional), Spanish, French, German

---

## Themes

- Located in `themes/` directory as JSON files
- Default: `themes/default.json`
- Over 20 themes available (catppuccin, tokyonight, gruvbox, etc.)

---

## Known Issues

### GCC 1.15 Compilation

If encountering compilation errors:
1. Downgrade GCC to 13.2.0 in MSYS2
2. Or disable `syntect` default features and use `default-fancy` instead
3. Lock GCC version in `/etc/pacman.conf`:
   ```
   IgnorePkg = mingw-w64-x86_64-gcc mingw-w64-x86_64-gcc-libs
   ```

---

## External Dependencies

- **GPUI:** `git = "https://github.com/zed-industries/zed"`
- **GPUI Component:** `git = "https://github.com/linruohan/gpui-component.git"`
- **Sea-ORM:** v1.1.19 with SQLite, chrono, JSON
- **rust-i18n:** v3.1.5
