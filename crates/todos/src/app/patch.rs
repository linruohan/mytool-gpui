use std::sync::Arc;

use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement};

#[derive(Debug)]
pub struct Patch {
    pub version: i32,
    pub description: &'static str,
    pub sql: &'static str,
}

#[derive(Debug)]
pub struct PatchManager {
    db: Arc<DatabaseConnection>,
    patches: Vec<Patch>,
}

impl PatchManager {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let patches = vec![
            Patch {
                version: 1,
                description: "Initial database schema",
                sql: include_str!("../../setup.sql"),
            },
            // 未来的补丁将添加在这里
        ];

        Self { db, patches }
    }

    pub async fn apply_patches(&self) -> Result<(), DbErr> {
        // 确保 db_version 表存在
        self.ensure_version_table().await?;

        // 获取当前版本
        let current_version = self.get_current_version().await?;

        // 检查是否有需要应用的补丁
        let mut has_patches_to_apply = false;
        for patch in &self.patches {
            if patch.version > current_version {
                has_patches_to_apply = true;
                break;
            }
        }

        // 如果没有需要应用的补丁，直接返回
        if !has_patches_to_apply {
            return Ok(());
        }

        // 打印当前版本
        tracing::info!("Current database version: {}", current_version);

        // 应用未应用的补丁
        for patch in &self.patches {
            if patch.version > current_version {
                tracing::info!("Applying patch version {}: {}", patch.version, patch.description);

                // 执行补丁 SQL
                for statement in patch.sql.split(';') {
                    let stmt = statement.trim();
                    if !stmt.is_empty() && !stmt.starts_with("--") && !stmt.starts_with("/*") {
                        tracing::debug!("Executing patch SQL: {}", stmt);
                        self.db
                            .execute(Statement::from_string(sea_orm::DbBackend::Sqlite, stmt))
                            .await?;
                    }
                }

                // 更新版本
                self.update_version(patch.version).await?;
                tracing::info!("Patch version {} applied successfully", patch.version);
            }
        }

        Ok(())
    }

    pub async fn get_current_version(&self) -> Result<i32, DbErr> {
        let result = self
            .db
            .query_one(Statement::from_string(
                sea_orm::DbBackend::Sqlite,
                "SELECT version FROM db_version ORDER BY version DESC LIMIT 1",
            ))
            .await?;

        match result {
            Some(row) => Ok(row.try_get::<i32>("", "version")?),
            None => Ok(0),
        }
    }

    async fn ensure_version_table(&self) -> Result<(), DbErr> {
        let create_table_sql = r#"
            CREATE TABLE IF NOT EXISTS db_version (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                version INTEGER NOT NULL,
                applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        "#;

        self.db
            .execute(Statement::from_string(sea_orm::DbBackend::Sqlite, create_table_sql))
            .await?;
        Ok(())
    }

    async fn update_version(&self, version: i32) -> Result<(), DbErr> {
        let insert_sql = format!("INSERT INTO db_version (version) VALUES ({})", version);
        self.db.execute(Statement::from_string(sea_orm::DbBackend::Sqlite, insert_sql)).await?;
        Ok(())
    }
}
