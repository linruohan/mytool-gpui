//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.12

use async_trait::async_trait;
use sea_orm::entity::prelude::*;
use sea_orm::{DbErr, Set};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "attachments")]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_type = "Text", indexed)]
    pub item_id: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub file_type: Option<String>,
    #[sea_orm(column_type = "Text", unique)]
    pub file_name: String,
    #[sea_orm(column_type = "Integer")]
    pub file_size: u64,
    #[sea_orm(column_type = "Text")]
    pub file_path: String,
}

#[derive(Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::items::Entity",
        from = "Column::ItemId",
        to = "super::items::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Items,
}

impl Related<super::items::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        let mut this = self;
        if insert {
            this.id = Set(Uuid::new_v4().to_string());
        }
        // 设置默认值
        if this.file_size.is_not_set() {
            this.file_size = Set(0);
        }

        Ok(this)
    }
}
