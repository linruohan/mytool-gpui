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

                // 🔧 关键修复：正确分割 SQL 语句，处理 TRIGGER 定义
                // TRIGGER 定义中包含多个分号，需要作为一个整体执行
                let statements = Self::split_sql_statements(patch.sql);

                for stmt in statements {
                    tracing::debug!("Executing patch SQL: {}", stmt);
                    self.db
                        .execute(Statement::from_string(sea_orm::DbBackend::Sqlite, stmt))
                        .await?;
                }

                // 更新版本
                self.update_version(patch.version).await?;
                tracing::info!("Patch version {} applied successfully", patch.version);
            }
        }

        Ok(())
    }

    /// 🔧 关键方法：正确分割 SQL 语句
    ///
    /// 问题：简单的按分号分割会破坏 TRIGGER 定义，因为 TRIGGER 内部包含多个分号
    /// 解决方案：跟踪括号和 BEGIN/END 块，确保 TRIGGER 定义保持完整
    fn split_sql_statements(sql: &str) -> Vec<String> {
        let mut statements = Vec::new();
        let mut current = String::new();
        let mut in_trigger_body = false; // 是否在 TRIGGER 的 BEGIN...END 块内
        let mut paren_depth = 0; // 括号深度

        for line in sql.lines() {
            let trimmed_line = line.trim();

            // 跳过空行和注释
            if trimmed_line.is_empty()
                || trimmed_line.starts_with("--")
                || trimmed_line.starts_with("/*")
            {
                continue;
            }

            // 检查是否进入 TRIGGER 定义（查找 "BEGIN" 关键字）
            if trimmed_line.contains("BEGIN") && !trimmed_line.contains("END") {
                in_trigger_body = true;
            }

            // 计算括号深度
            for ch in trimmed_line.chars() {
                if ch == '(' {
                    paren_depth += 1;
                } else if ch == ')' {
                    paren_depth -= 1;
                }
            }

            // 添加到当前语句
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(trimmed_line);

            // 检查是否结束 TRIGGER 定义（查找 "END;"）
            if in_trigger_body && trimmed_line == "END;" {
                in_trigger_body = false;
                // 移除末尾的分号，添加完整语句
                if current.ends_with(';') {
                    current.pop();
                }
                statements.push(current.trim().to_string());
                current = String::new();
                continue;
            }

            // 如果不在 TRIGGER 体内，且以分号结尾，且括号已闭合，则是一个完整语句
            if !in_trigger_body && trimmed_line.ends_with(';') && paren_depth == 0 {
                // 移除末尾的分号
                if current.ends_with(';') {
                    current.pop();
                }
                statements.push(current.trim().to_string());
                current = String::new();
            }
        }

        // 处理剩余的语句（如果没有分号结尾）
        if !current.is_empty() {
            let stmt = current.trim().to_string();
            if !stmt.is_empty() && !stmt.starts_with("--") {
                statements.push(stmt);
            }
        }

        statements
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
