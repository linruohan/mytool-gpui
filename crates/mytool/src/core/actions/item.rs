use std::sync::Arc;

use gpui::App;
use todos::entity::ItemModel;

use super::{delete_item_optimistic, update_item_optimistic};

/// 修改 item（委托给乐观更新路径）
pub fn update_item(item: Arc<ItemModel>, cx: &mut App) {
    update_item_optimistic(item, cx);
}

/// 删除 item（委托给乐观更新路径）
pub fn delete_item(item: Arc<ItemModel>, cx: &mut App) {
    delete_item_optimistic(item, cx);
}
