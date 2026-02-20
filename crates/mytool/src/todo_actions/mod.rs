//! todo_actions 层职责说明
//!
//! 本模块负责处理业务操作，调用 service 层进行数据库操作，然后更新状态。
//!
//! ## 架构说明
//!
//! ```mermaid
//! graph TB
//!     subgraph "调用方"
//!         V[视图层 Views]
//!     end
//!
//!     subgraph "todo_actions 层"
//!         A1[add_item]
//!         A2[update_item]
//!         A3[delete_item]
//!         A4[completed_item]
//!         A5[store_actions]
//!         A6[incremental_actions]
//!     end
//!
//!     subgraph "service 层"
//!         S1[add_item]
//!         S2[mod_item]
//!         S3[del_item]
//!         S4[finish_item]
//!     end
//!
//!     subgraph "状态层"
//!         T1[ItemState]
//!         T2[TodoStore]
//!     end
//!
//!     V --> A1 & A2 & A3 & A4
//!     A1 & A2 & A3 & A4 --> S1 & S2 & S3 & S4
//!     A1 & A2 & A3 & A4 --> T1
//!     A5 --> T2
//!     A6 --> T2
//! ```
//!
//! ## 迁移指南
//!
//! ### 旧方式（多次数据库查询）
//!
//! ```ignore
//! // 每次 update_item 会触发 4 次数据库查询
//! update_item(item, cx);
//! ```
//!
//! ### 新方式（单次数据库查询）
//!
//! ```ignore
//! // 使用 store_actions，只触发 1 次数据库查询
//! update_item_in_store(item, cx, db).await;
//! ```
//!
//! ### 增量更新方式（推荐，性能最优）
//!
//! ```ignore
//! // 使用 incremental_actions，只更新单条数据，不刷新全部
//! update_item_incremental(item, cx, db).await;
//! ```
//!
//! ## 建议
//!
//! 1. **新代码优先使用 `incremental_actions` 模块**（性能最优）
//! 2. 旧代码逐步迁移到 `incremental_actions`
//! 3. `store_actions` 保留用于需要全量刷新的场景
//! 4. 保持向后兼容，旧代码继续工作

mod attachment;
mod batch_operations;
mod item;
mod label;
mod project_item;
mod reminder;
mod section;
mod store_actions;

pub use attachment::*;
// 批量操作（高性能）
#[allow(unused_imports)]
pub use batch_operations::*;
// 增量更新操作（推荐，性能最优）
pub use item::*;
pub use label::*;
pub use project_item::*;
pub use reminder::*;
pub use section::*;
// 全量刷新操作（保留用于兼容）
#[allow(unused_imports)]
pub use store_actions::*;
