pub mod imp;
pub mod schema;

use crate::services::database::Database;
use anyhow::{Ok, Result};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, PooledConnection};
pub(crate) use imp::{NewTodo, Todo, UpdateTodo};
use schema::todos;
use schema::todos::id;
impl Todo {
    pub fn new(title: String, content: String) -> NewTodo {
        NewTodo { title, content }
    }

    pub async fn create_todo(&self, new_todo: NewTodo) -> Result<()> {
        let mut conn = Database::get_conn();
        diesel::insert_into(todos::table)
            .values(&new_todo)
            .returning(Todo::as_returning())
            .execute(&mut conn)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }
    pub async fn todos(&self) -> Result<Vec<Todo>> {
        let mut conn = get_db()?;
        let results = todos::table
            .load::<Todo>(&mut conn)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(results)
    }

    pub async fn get_todo(&self, todo_id: i32) -> Result<Todo> {
        let mut conn = get_db()?;
        let todo = todos::table
            .filter(id.eq(todo_id))
            .first::<Todo>(&mut conn)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(todo)
    }

    pub async fn update_todo(&self, todo_id: i32, update_todo: UpdateTodo) -> Result<()> {
        let mut conn = get_db()?;
        diesel::update(todos::table.filter(id.eq(todo_id)))
            .set(&update_todo)
            .returning(Todo::as_returning())
            .get_result(&mut conn)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }

    pub async fn delete_todo(&self, todo_id: i32) -> Result<()> {
        let mut conn = get_db()?;
        let _ = diesel::delete(todos::table.filter(id.eq(todo_id)))
            .execute(&mut conn)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(())
    }
}
