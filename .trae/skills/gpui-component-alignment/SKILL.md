---
name: "gpui-component-alignment"
description: "诊断并修复 mytool-gpui 与 gpui-component 生态（gpui-pre / gpui-fps）对齐导致的编译错误与警告。当出现 gpui 多版本冲突、GPUI 依赖升级后大量 E0599/E0277/E0308 级联错误、profile spec 不匹配、或需参考官方 story crate 用法时调用。"
---

# GPUI Component 生态对齐与编译排障

本 skill 沉淀 mytool-gpui 工作区与 `gpui-component`（linruohan/gpui-component）生态对齐的
排查流程，覆盖依赖版本冲突诊断、包名映射、API 参考定位和警告清零模式。

## 一、何时调用

- 编译出现**几十上百个** trait/方法解析错误（E0599 no method、E0277 trait bound、E0308 类型不匹配），
  且错误信息中出现 `there are multiple different versions of crate gpui in the dependency graph`
- 升级/切换 `gpui`、`gpui-component`、`gpui-kit`、`gpui-fps` 后编译失败
- 出现 `profile package spec ... did not match any packages` 警告
- 需要确认 gpui-component 官方 API 用法（以 story crate 为权威参考）
- GPUI 相关依赖的 warning 清零

## 二、核心事实：包名映射表

gpui-component 新版已从 zed git 仓库的 gpui 切换到 **crates.io 上的 gpui-pre 快照系列**。
注意区分 **crate 包名**（Cargo.toml 中 `package =`）与 **lib 名**（源码 `use` 的名字）：

| Cargo.toml 依赖键 | 实际包名 (package) | 源码 lib 名 (use) | 备注 |
|---|---|---|---|
| `gpui` | `gpui-pre` | `gpui` | lib 名不变，**源码 use 零改动**；需 `profiler` feature 才有 FrameTiming |
| `gpui_platform` | `gpui-pre-platform` | `gpui_platform` | features: font-kit/x11/wayland/runtime_shaders |
| `gpui_macros` | `gpui-pre-macros` | `gpui_macros` | 仅在显式使用其宏时才需要声明 |
| `gpui-fps` | git 仓库内 crate | `gpui_fps` | 用官方 git 源，不要维护本地 fork |

workspace 依赖标准写法（根 Cargo.toml `[workspace.dependencies]`）：

```toml
gpui = { package = "gpui-pre", version = "0.3.3", features = ["profiler"] }
gpui_platform = { package = "gpui-pre-platform", version = "0.3.3", features = [
    "font-kit", "x11", "wayland", "runtime_shaders",
] }
gpui-component = { git = "https://github.com/linruohan/gpui-component.git", features = ["tree-sitter-languages"] }
gpui-kit = { git = "https://github.com/linruohan/gpui-component.git" }
gpui-fps = { git = "https://github.com/linruohan/gpui-component.git" }
```

## 三、典型故障：双版本 gpui 冲突

**特征**：一次报出数百个错误（如 805 个），典型形态：
- `no method named 'w_full' found for struct gpui::elements::div::Div`
- 注释里同时列出 `gpui-pre-x.y.z/src/styled.rs`（expected trait）和 zed git checkout 路径（imported trait）
- `register_panel` 等函数参数类型不匹配（两个不同的 `App` 类型）

**根因**：workspace 用 zed git 版 gpui，而新版 gpui-component 依赖 crates.io 的 gpui-pre，
依赖图中存在两套 gpui 类型系统，trait/方法无法跨版本解析。

**诊断命令**（PowerShell）：

```powershell
# 查谁引入了 gpui-pre / zed git gpui
cargo tree -p mytool -i gpui-pre
cargo tree -p mytool | Select-String "gpui"
```

**修复要点**：
1. 按第二节映射表把 workspace 依赖统一切到 gpui-pre 系列
2. **所有内部 fork / path 依赖必须同步切换**（曾因 `third-party/gpui-fps` 仍引用 zed git gpui
   而再次拉入第二份 gpui）；能直接用官方 git 源的就删除本地 fork
3. 切换后删除为旧 fork 服务的 `[patch."..."]` 段
4. lib 名不变 → 业务代码通常零改动

## 四、权威参考：本地 gpui-component checkout

参考工程位于 `D:\codehub\gpui-component`（与 CI 拉取的 git 源同源）：

| 用途 | 路径 |
|---|---|
| 依赖写法权威来源 | `D:\codehub\gpui-component\Cargo.toml`（`[workspace.dependencies]`） |
| API 用法权威示例 | `D:\codehub\gpui-component\crates\story\src\` |
| FPS 监控用法 | story 中 `use gpui_fps::fps_monitor;` + `.when(show_fps, \|this\| this.child(fps_monitor(window, cx)))` |
| fps crate 本体 | `D:\codehub\gpui-component\crates\fps\` |

排查未知 API 时，先 grep story crate：
`Grep(pattern="用法关键词", path="D:\\codehub\\gpui-component\\crates\\story")`

## 五、profile spec 规则

`[profile.dev.package]` 的 key 必须是**真实 crate 包名**，不是 lib 名、也不是依赖键名。
切换 gpui-pre 后旧 spec 会失效并报警告：

```toml
[profile.dev.package]
"*" = { opt-level = 3 }
gpui-pre = { opt-level = 3 }            # ✅ 不是 gpui
gpui-pre-platform = { opt-level = 3 }   # ✅ 不是 gpui_platform
gpui-pre-macros = { opt-level = 3 }     # ✅ 不是 gpui_macros
```

## 六、Warning 清零模式库

| 警告 | 模式 | 处理 |
|---|---|---|
| `unused workspace dependency` | 根 Cargo.toml 声明了但无 crate 用 `.workspace = true` | grep 全部 `**/Cargo.toml` 确认无引用后删除声明 |
| `unused field in workspace.package` | `publish`/`edition` 无成员继承 | 删除该字段（成员各自声明即可） |
| `unused dependency winresource` | Windows 构建实际用 `embed-resource`（build.rs 中 `embed_resource::compile`） | 删 build-dependencies 与 machete ignore 中的 winresource |
| `dead_code`（builder setter） | 预留扩展点：字段在 render 中使用、setter 暂无调用方 | `#[allow(dead_code, reason = "预留 builder 扩展点")]` + 注释说明 |
| `clippy::too_many_arguments` | GPUI 渲染函数天然聚合视图/数据/样式/交互 | `#[allow(clippy::too_many_arguments, reason = "...")]`，不强行拆参数对象 |
| `clippy::needless_bool` | `if x { return true; } false` | 直接返回布尔表达式 `x`（等价重构） |
| `clippy::await_holding_lock` | `RwLockReadGuard`/`MutexGuard` 跨 `.await` 持有 | 用小块 `{ let g = lock.read()...; 拷贝需要的数据 }` 先释放锁再 await |
| `cargo::non_kebab_case_bins` | 包名 `my_test` | 改名 `my-test`（先 grep 确认无包名引用） |
| `missing_lints_inheritance` | 成员 crate 未继承 lint | 加 `[lints]\nworkspace = true` |

**test crate 专属坑**：
- `#[tokio::main]` **不能**加在被 `runtime.block_on()` 调用的 async fn 上——该宏会把 async fn
  改写成同步 fn，调用处 `.await` 会报 `Result is not a future`。入口 fn 才用它。
- sea-orm 1.1.x：按列索引取值用 `row.try_get_by_index(idx)`；
  `try_get(pre, col)` 需要"表前缀 + 列名"两个参数。
- `std::process::Command` 在本仓库被禁用（clippy.toml），用 `smol::process::Command`。

## 七、验证流程

```powershell
cargo build --workspace              # 编译 + cargo 级警告
cargo clippy --workspace --message-format short   # clippy lint（build 缓存会掩盖 clippy 警告）
```

注意：
- **clippy 对未改动 crate 会复用缓存**，可能隐藏警告；修改文件或 `cargo clean -p <crate>` 后重跑
- `cargo check -p <crate>` 快速验证单 crate；`cargo tree -i <pkg>` 定位依赖来源
- PowerShell 不支持 `sort -rn` 等 Unix 参数，用 `Select-String`/`Select-Object -First N`

## 八、操作纪律（教训）

- **不要并行 Edit 同一个文件**：多个 Edit 并行写同一文件会产生竞态覆盖，部分改动被静默回滚。
  同文件的多处修改必须**顺序执行**，改完后 Read 复核最终状态。
- 依赖删除前先 grep 全仓库确认（`**/Cargo.toml` 中的 `.workspace = true` 引用）。
- 最小化修改：优先改 Cargo.toml 配置，源码能不动就不动（gpui-pre 的 lib 名兼容使这成为可能）。
