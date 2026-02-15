//! Item-Label 关联表实体
//!
//! 此表用于维护 Item 和 Label 之间的多对多关系
//! 替代原有的 items.labels 字段（分号分隔的字符串存储）

use async_trait::async_trait;
use sea_orm::{DbErr, Set, entity::prelude::*};
use serde::{Deserialize, Serialize};

/// Item-Label 关联表实体
///
/// 主键为复合主键 (item_id, label_id)
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "item_labels")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    /// 关联的 Item ID
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub item_id: String,

    /// 关联的 Label ID
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub label_id: String,

    /// 关联创建时间
    pub created_at: chrono::NaiveDateTime,
}

/// 定义实体关系
#[derive(Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联到 Items 表
    #[sea_orm(
        belongs_to = "super::items::Entity",
        from = "Column::ItemId",
        to = "super::items::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Item,

    /// 关联到 Labels 表
    #[sea_orm(
        belongs_to = "super::labels::Entity",
        from = "Column::LabelId",
        to = "super::labels::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Label,
}

/// 实现关联 trait，方便通过 ItemLabel 查询关联的 Item
impl Related<super::items::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Item.def()
    }
}

/// 实现关联 trait，方便通过 ItemLabel 查询关联的 Label
impl Related<super::labels::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Label.def()
    }
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// 保存前自动设置创建时间
    async fn before_save<C>(self, db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        let mut this = self;

        if insert {
            // 新记录，设置创建时间
            let now = chrono::Utc::now().naive_utc();
            this.created_at = Set(now);
        }

        Ok(this)
    }
}
