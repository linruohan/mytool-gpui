use crate::Source;
use crate::schema::{attachments, items, labels, projects, queue, reminders, sections, sources};
use diesel::QueryDsl;
use diesel::SqliteConnection;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::{
    Table,
    expression::BoxableExpression,
    query_builder::{AsChangeset, IntoUpdateTarget},
    sql_types::Bool,
};
use dotenvy::dotenv;
use once_cell::sync::OnceCell;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use super::{Attachment, BaseTrait, Item, Label, Project, Reminder};
use super::{Queue, Section};
pub type DbPool = Arc<r2d2::Pool<ConnectionManager<SqliteConnection>>>;
// 定义全局的数据库连接池
pub static DB_POOL: OnceCell<DbPool> = OnceCell::new();
pub type PooledConnection = r2d2::PooledConnection<ConnectionManager<SqliteConnection>>;
pub struct Database {}
fn get_db_pool() -> &'static DbPool {
    DB_POOL.get_or_init(|| {
        dotenv().ok(); // 加载 .env 文件
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        let pool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create database pool");
        Arc::new(pool)
    })
}

/// 从连接池中获取一个连接
// pub fn get_conn(pool: &DbPool) -> PooledConnection {
//     pool.get().expect("Failed to get connection from pool")
// }
impl Database {
    pub fn default() -> Database {
        Self {}
    }
    /// 从连接池中获取一个连接
    pub fn get_conn(&self) -> PooledConnection {
        let pool = get_db_pool().clone();
        pool.get().expect("Failed to get connection from pool")
    }

    fn update<F, S>(&self, filter_source: F, set: S) -> bool
    where
        F: diesel::query_builder::IntoUpdateTarget + diesel::associations::HasTable + Send,
        S: diesel::AsChangeset<Target = F::Table> + Send,
        F::Table: Table,
        F::WhereClause: Send,
    {
        // let mut conn = self.get_conn();
        // diesel::update(filter_source)
        //     .set(set)
        //     .execute(&mut conn)
        //     .is_ok()
        todo!();
        false
    }
    pub fn clear_database(&self, pool: DbPool) -> Result<(), String> {
        // 1 关闭连接池
        let _ = drop(pool);
        // 2 删除文件
        // 2.1 获取数据库 URL
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        // 2.2 从数据库 URL 中提取文件路径
        let file_path = database_url
            .strip_prefix("sqlite://")
            .ok_or("Invalid database URL format")?;

        // 2.3 检查文件是否存在
        if !Path::new(file_path).exists() {
            return Err("Database file does not exist".to_string());
        }
        // 3.4删除文件
        fs::remove_file(file_path).map_err(|e| e.to_string())?;

        println!("Database file deleted: {}", file_path);
        Ok(())
    }

    // sources
    pub fn get_sources_collection(&self) -> Vec<Source> {
        let mut conn = self.get_conn();
        diesel::sql_query(
            r#"
           SELECT * FROM Sources ORDER BY child_order;
             "#,
        )
        .load::<Source>(&mut conn)
        .expect("Failed to get Sources")
    }
    pub fn insert_source(&self, source: Source) -> bool {
        let mut conn = self.get_conn();
        diesel::insert_into(sources::table)
            .values(&source)
            .execute(&mut conn)
            .is_ok()
    }
    pub fn delete_source(&self, source: Source) -> bool {
        let mut conn = self.get_conn();
        diesel::delete(sources::table.filter(sources::id.eq(&source.id)))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn update_source(&self, source: &Source) -> bool {
        let mut conn = self.get_conn();
        diesel::update(sources::table.filter(sources::id.eq(&source.id)))
            .set((
                sources::source_type.eq(&source.source_type),
                sources::display_name.eq(&source.display_name),
                sources::added_at.eq(&source.added_at),
                sources::updated_at.eq(&source.updated_at),
                sources::is_visible.eq(&source.is_visible),
                sources::child_order.eq(&source.child_order),
                sources::sync_server.eq(&source.sync_server),
                sources::last_sync.eq(&source.last_sync),
                sources::data.eq(&source.data),
            ))
            .execute(&mut conn)
            .is_ok()
    }
    // items
    pub fn get_items_collection(&self) -> Vec<Item> {
        let mut conn = self.get_conn();
        diesel::sql_query(
            r#"
           SELECT * FROM items WHERE is_deleted = 0;
             "#,
        )
        .load::<Item>(&mut conn)
        .expect("Failed to get Items")
    }
    pub fn insert_item(&self, item: &Item) -> bool {
        let mut conn = self.get_conn();
        diesel::insert_into(items::table)
            .values(item)
            .execute(&mut conn)
            .is_ok()
    }
    pub fn get_item_by_id(&self, id: String) -> Item {
        let mut conn = self.get_conn();
        items::table
            .filter(items::id.eq(id))
            .first::<Item>(&mut conn)
            .expect("failed get item with id {id}")
    }
    pub fn delete_item(&self, item: &Item) -> bool {
        let mut conn = self.get_conn();
        diesel::delete(items::table.filter(items::id.eq(&item.id)))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn update_item(&self, item: &Item) -> bool {
        let mut conn = self.get_conn();
        diesel::update(items::table.filter(items::id.eq(&item.id)))
            .set((
                items::content.eq(&item.content),
                items::description.eq(&item.description),
                items::due.eq(&item.due),
                items::added_at.eq(&item.added_at),
                items::completed_at.eq(&item.completed_at),
                items::updated_at.eq(&item.updated_at),
                items::section_id.eq(&item.section_id),
                items::project_id.eq(&item.project_id),
                items::parent_id.eq(&item.parent_id),
                items::priority.eq(&item.priority),
                items::child_order.eq(&item.child_order),
                items::checked.eq(&item.checked),
                items::is_deleted.eq(&item.is_deleted),
                items::day_order.eq(&item.day_order),
                items::collapsed.eq(&item.collapsed),
                items::pinned.eq(&item.pinned),
                items::labels.eq(&item.labels),
                items::extra_data.eq(&item.extra_data),
                items::item_type.eq(&item.item_type),
            ))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn move_item(&self, item: &Item) -> bool {
        let mut conn = self.get_conn();
        diesel::update(items::table.filter(items::id.eq(&item.id)))
            .set((
                items::section_id.eq(&item.section_id),
                items::project_id.eq(&item.project_id),
                items::parent_id.eq(&item.parent_id),
            ))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn complete_item(&self, item: &Item) -> bool {
        let mut conn = self.get_conn();
        diesel::update(
            items::table.filter(items::id.eq(&item.id).or(items::parent_id.eq(&item.id))),
        )
        .set((
            items::completed_at.eq(&item.completed_at),
            items::checked.eq(&item.checked),
        ))
        .execute(&mut conn)
        .is_ok()
    }
    pub fn update_project_item_id(&self, cur_id: &str, new_id: &str) -> bool {
        let mut conn = self.get_conn();
        diesel::update(items::table.filter(items::project_id.eq(&cur_id)))
            .set(items::project_id.eq(&new_id))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn update_section_item_id(&self, cur_id: &str, new_id: &str) -> bool {
        let mut conn = self.get_conn();
        diesel::update(items::table.filter(items::section_id.eq(&cur_id)))
            .set(items::section_id.eq(&new_id))
            .execute(&mut conn)
            .is_ok()
    }

    pub fn update_item_id(&self, cur_id: &str, new_id: &str) -> bool {
        let mut conn = self.get_conn();
        diesel::update(items::table.filter(items::id.eq(&cur_id)))
            .set(items::id.eq(&new_id))
            .execute(&mut conn)
            .is_ok()
    }

    pub fn update_item_child_id(&self, cur_id: &str, new_id: &str) -> bool {
        let mut conn = self.get_conn();
        diesel::update(items::table.filter(items::parent_id.eq(&cur_id)))
            .set(items::parent_id.eq(&new_id))
            .execute(&mut conn)
            .is_ok()
    }
    // attachments
    pub fn get_attachments_collection(&self) -> Vec<Attachment> {
        let mut conn = self.get_conn();
        diesel::sql_query(
            r#"
          SELECT * FROM attachments;
             "#,
        )
        // .execute(&mut conn)
        .load::<Attachment>(&mut conn)
        .expect("Failed to get Sources")
    }
    pub fn insert_attachment(&self, attachment: &Attachment) -> bool {
        let mut conn = self.get_conn();
        diesel::insert_into(attachments::table)
            .values(attachment)
            .execute(&mut conn)
            .is_ok()
    }
    pub fn delete_attachment(&self, attachment: &Attachment) -> bool {
        let mut conn = self.get_conn();
        diesel::delete(attachments::table.filter(attachments::id.eq(&attachment.id)))
            .execute(&mut conn)
            .is_ok()
    }
    // projects
    pub(crate) fn get_projects_collection(&self) -> Vec<Project> {
        let mut conn = self.get_conn();
        diesel::sql_query(
            r#"
          SELECT * FROM Projects WHERE is_deleted = 0 ORDER BY child_order;
             "#,
        )
        .load::<Project>(&mut conn)
        .expect("Failed to get Project")
    }
    pub fn insert_project(&self, project: &Project) -> bool {
        let mut conn = self.get_conn();
        diesel::insert_into(projects::table)
            .values(project)
            .execute(&mut conn)
            .is_ok()
    }
    pub fn delete_project(&self, project: &Project) -> bool {
        let mut conn = self.get_conn();
        diesel::delete(projects::table.filter(projects::id.eq(&project.id)))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn delete_project_db(&self, project: &Project) -> bool {
        let mut conn = self.get_conn();
        let b1 = diesel::delete(projects::table.filter(projects::id.eq(&project.id)))
            .execute(&mut conn)
            .is_ok();
        let b2 = diesel::delete(items::table.filter(items::project_id.eq(&project.id)))
            .execute(&mut conn)
            .is_ok();
        b1 && b2
    }

    pub fn update_project(&self, project: Project) -> bool {
        let mut conn = self.get_conn();
        diesel::update(projects::table.filter(projects::id.eq(&project.id)))
            .set((
                projects::parent_id.eq(&project.parent_id),
                projects::name.eq(&project.name),
                projects::source_id.eq(&project.source_id),
                projects::color.eq(&project.color),
                projects::backend_type.eq(&project.backend_type),
                projects::inbox_project.eq(&project.inbox_project),
                projects::team_inbox.eq(&project.team_inbox),
                projects::child_order.eq(&project.child_order),
                projects::is_deleted.eq(&project.is_deleted),
                projects::is_archived.eq(&project.is_archived),
                projects::is_favorite.eq(&project.is_favorite),
                projects::shared.eq(&project.shared),
                projects::view_style.eq(&project.view_style),
                projects::sort_order.eq(&project.sort_order),
                projects::collapsed.eq(&project.collapsed),
                projects::icon_style.eq(&project.icon_style),
                projects::emoji.eq(&project.emoji),
                projects::show_completed.eq(&project.show_completed),
                projects::description.eq(&project.description),
                projects::due_date.eq(&project.due_date),
                projects::inbox_section_hidded.eq(&project.inbox_section_hidded),
                projects::sync_id.eq(&project.sync_id),
            ))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn archive_project(&self, project: Project) -> bool {
        let mut conn = self.get_conn();
        diesel::update(projects::table.filter(projects::id.eq(&project.id)))
            .set((projects::is_archived.eq(&project.is_archived),))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn update_project_id(&self, cur_id: &str, new_id: &str) -> bool {
        let mut conn = self.get_conn();
        diesel::update(projects::table.filter(projects::id.eq(&cur_id)))
            .set(projects::id.eq(&new_id))
            .execute(&mut conn)
            .is_ok()
    }

    // labels
    pub(crate) fn get_labels_collection(&self) -> Vec<Label> {
        let mut conn = self.get_conn();
        diesel::sql_query(
            r#"
          SELECT * FROM Labels;
             "#,
        )
        .load::<Label>(&mut conn)
        .expect("Failed to get Label")
    }
    pub fn get_labels_ids(&self, labels: Vec<Label>) -> String {
        labels
            .iter()
            .map(|label| label.id_string())
            .collect::<Vec<_>>()
            .join(";")
    }
    pub fn insert_label(&self, label: Label) -> bool {
        let mut conn = self.get_conn();
        diesel::insert_into(labels::table)
            .values(&label)
            .execute(&mut conn)
            .is_ok()
    }
    pub fn delete_label(&self, label: Label) -> bool {
        let mut conn = self.get_conn();
        diesel::delete(labels::table.filter(labels::id.eq(&label.id)))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn update_label(&self, label: Label) -> bool {
        let mut conn = self.get_conn();
        diesel::update(labels::table.filter(labels::id.eq(&label.id)))
            .set((
                labels::name.eq(&label.name),
                labels::color.eq(&label.color),
                labels::item_order.eq(&label.item_order),
                labels::is_deleted.eq(&label.is_deleted),
                labels::is_favorite.eq(&label.is_favorite),
                labels::backend_type.eq(&label.backend_type),
                labels::source_id.eq(&label.source_id),
            ))
            .execute(&mut conn)
            .is_ok()
    }
    // reminders
    pub(crate) fn get_reminders_collection(&self) -> Vec<Reminder> {
        let mut conn = self.get_conn();
        diesel::sql_query(
            r#"
          SELECT id, item_id, type, due, mm_offset FROM Reminders;
             "#,
        )
        .load::<Reminder>(&mut conn)
        .expect("Failed to get Label")
    }
    pub fn insert_reminder(&self, reminder: Reminder) -> bool {
        let mut conn = self.get_conn();
        diesel::insert_into(reminders::table)
            .values(&reminder)
            .execute(&mut conn)
            .is_ok()
    }
    pub fn get_reminders_by_item_id(&self, id: String) -> Vec<Reminder> {
        let mut conn = self.get_conn();
        diesel::sql_query(
            r#"
            SELECT id, item_id, type, due, mm_offset FROM Reminders WHERE item_id=$item_id;
            "#,
        )
        .bind::<diesel::sql_types::Text, _>(id)
        .load::<Reminder>(&mut conn)
        .expect("Failed to get reminders")
    }
    pub fn delete_reminder(&self, reminder: &Reminder) -> bool {
        let mut conn = self.get_conn();
        diesel::delete(reminders::table.filter(reminders::item_id.eq(&reminder.id)))
            .execute(&mut conn)
            .is_ok()
    }
    // sections
    pub(crate) fn get_sections_collection(&self) -> Vec<Section> {
        let mut conn = self.get_conn();
        diesel::sql_query(
            r#"
          SELECT * FROM Sections WHERE is_deleted = 0;
             "#,
        )
        .load::<Section>(&mut conn)
        .expect("Failed to get Section")
    }
    pub fn insert_section(&self, section: &Section) -> bool {
        let mut conn = self.get_conn();
        diesel::insert_into(sections::table)
            .values(section)
            .execute(&mut conn)
            .is_ok()
    }
    pub fn delete_section(&self, section: &Section) -> bool {
        let mut conn = self.get_conn();
        diesel::delete(sections::table.filter(sections::id.eq(&section.id)))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn update_section(&self, section: &Section) -> bool {
        let mut conn = self.get_conn();
        diesel::update(sections::table.filter(sections::id.eq(&section.id)))
            .set((
                sections::name.eq(&section.name),
                sections::archived_at.eq(&section.archived_at),
                sections::added_at.eq(&section.added_at),
                sections::project_id.eq(&section.project_id),
                sections::section_order.eq(&section.section_order),
                sections::collapsed.eq(&section.collapsed),
                sections::is_deleted.eq(&section.is_deleted),
                sections::is_archived.eq(&section.is_archived),
                sections::color.eq(&section.color),
                sections::description.eq(&section.description),
                sections::hidded.eq(&section.hidded),
            ))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn move_section(&self, section: &Section, project_id: &str) -> bool {
        let mut conn = self.get_conn();
        diesel::update(sections::table.filter(sections::id.eq(&section.id)))
            .set((sections::project_id.eq(project_id),))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn move_section_items(&self, section: &Section) -> bool {
        let mut conn = self.get_conn();
        diesel::update(items::table.filter(items::section_id.eq(&section.id)))
            .set((items::project_id.eq(section.project_id.as_deref()),))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn archive_section(&self, section: &Section) -> bool {
        let mut conn = self.get_conn();
        diesel::update(sections::table.filter(sections::id.eq(&section.id)))
            .set((sections::is_archived.eq(section.is_archived),))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn update_project_section_id(&self, cur_id: &str, new_id: &str) -> bool {
        let mut conn = self.get_conn();
        diesel::update(sections::table.filter(sections::project_id.eq(&cur_id)))
            .set(sections::project_id.eq(&new_id))
            .execute(&mut conn)
            .is_ok()
    }
    pub fn update_section_id(&self, cur_id: &str, new_id: &str) -> bool {
        let mut conn = self.get_conn();
        diesel::update(sections::table.filter(sections::id.eq(&cur_id)))
            .set(sections::id.eq(&new_id))
            .execute(&mut conn)
            .is_ok()
    }
    // queue
    pub(crate) fn get_all_queue(&self) -> Vec<Queue> {
        let mut conn = self.get_conn();
        diesel::sql_query(
            r#"
          SELECT * FROM Queue ORDER BY date_added;
             "#,
        )
        .load::<Queue>(&mut conn)
        .expect("Failed to get Queue")
    }
    pub fn insert_queue(&self, new_queue: Queue) -> bool {
        let mut conn = self.get_conn();
        diesel::insert_into(queue::table)
            .values(&new_queue)
            .execute(&mut conn)
            .is_ok()
    }
}
