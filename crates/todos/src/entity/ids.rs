use sea_orm::{ActiveValue, Set};
use uuid::Uuid;

/// 插入时保留调用方已分配的主键；仅在空值时生成 UUID。
pub(crate) fn assign_id_if_empty(id: &mut ActiveValue<String>) {
    let empty = match id {
        ActiveValue::Set(value) | ActiveValue::Unchanged(value) => value.is_empty(),
        ActiveValue::NotSet => true,
    };
    if empty {
        *id = Set(Uuid::new_v4().to_string());
    }
}
