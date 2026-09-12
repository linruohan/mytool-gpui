use std::time::Duration;

use sea_orm::{DatabaseConnection, EntityTrait};

use crate::{entity::prelude::*, error::TodoError};

fn is_ephemeral_id(id: &str) -> bool {
    id.is_empty() || id.starts_with("temp_")
}

async fn wait_for_row<E>(db: &DatabaseConnection, id: &str, entity: &str) -> Result<(), TodoError>
where
    E: EntityTrait,
    E::PrimaryKey: sea_orm::PrimaryKeyTrait,
    <E::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<String>,
{
    if is_ephemeral_id(id) {
        return Err(TodoError::not_found(entity));
    }

    const ATTEMPTS: u32 = 20;
    for attempt in 0..ATTEMPTS {
        if E::find_by_id(id.to_string()).one(db).await?.is_some() {
            return Ok(());
        }
        if attempt + 1 < ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    Err(TodoError::not_found(entity))
}

/// 子表写入前等待任务已落盘，避免 item_id 外键失败。
pub async fn wait_for_item(db: &DatabaseConnection, item_id: &str) -> Result<(), TodoError> {
    wait_for_row::<ItemEntity>(db, item_id, "Item").await
}

/// 分区写入前等待项目已落盘，避免 project_id 外键失败。
pub async fn wait_for_project(db: &DatabaseConnection, project_id: &str) -> Result<(), TodoError> {
    wait_for_row::<ProjectEntity>(db, project_id, "Project").await
}
