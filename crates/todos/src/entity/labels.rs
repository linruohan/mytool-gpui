//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.12

use async_trait::async_trait;
use sea_orm::entity::prelude::*;
use sea_orm::{DbErr, Set};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "labels")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_type = "Text", unique)]
    pub name: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub color: String,
    pub item_order: i32,
    pub is_deleted: bool,
    pub is_favorite: bool,
    #[sea_orm(column_type = "Text", nullable)]
    pub backend_type: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub source_id: Option<String>,
}

#[derive(Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

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

        Ok(this)
    }
}
