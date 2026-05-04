# AGENTS.md - MyTool-GPUI

## Build

```bash
# Debug build
cargo build

# Release build  
cargo build --release

# Single crate
cargo build -p mytool
```

## Lint & Format

```bash
# Format
cargo fmt --all

# Auto-fix all clippy
cargo clippy --fix --allow-dirty --allow-staged

# By priority
cargo clippy -- -D clippy::correctness    # Fix correctness first
cargo clippy -- -W clippy::style      # Then style

# Per crate
cargo clippy --fix --lib -p todos --allow-dirty
```

## Test

```bash
# All tests
cargo test

# Single test
cargo test test_name

# Specific crate
cargo test -p todos

# With output
cargo test -- --nocapture
```

## Critical Constraints

- **GCC 1.15**: Causes compilation failure. If encountered, downgrade to 13.2.0 or set `syntect = { default-features = false, features = ["default-fancy"] }`
- **`std::process::Command`**: Disallowed - use `smol::process::Command` instead
- **`serde_json::from_reader`**: Disallowed - use `from_slice` instead

## Architecture

```
mytool (GUI) → Services → Repositories → Sea-ORM → SQLite
```

**Crates:**
- `crates/mytool` - GPUI application
- `crates/todos` - Core logic + entities
- `crates/gconfig` - Config management

## Key Config Files

| File | Purpose |
|------|---------|
| `.cargo/config.toml` | Build settings, sccache, linker |
| `clippy.toml` | Disallowed methods/types |
| `rustfmt.toml` | Import grouping, line width 100 |

## External Dependencies (git)

- GPUI: `https://github.com/zed-industries/zed`
- gpui-component: `https://github.com/linruohan/gpui-component.git`