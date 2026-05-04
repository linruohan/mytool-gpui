//! 动作执行器 - 统一的错误处理和执行流程
//!
//! 提供统一的动作执行框架，封装验证、执行、错误处理流程
//!
//! ## 优化特性
//! - **统一错误处理**: 所有动作使用相同的错误处理模式
//! - **验证前置**: 在执行前进行输入验证
//! - **日志记录**: 自动记录操作成功/失败
//! - **代码复用**: 减少重复的错误处理代码

use std::{future::Future, pin::Pin, sync::Arc};

use gpui::{App, AsyncApp};
use todos::error::TodoError;
use tracing::{error, info};

use crate::core::{
    error_handler::{AppError, ErrorHandler, validation},
    state::ErrorNotifier,
};

/// 动作执行器 - 封装通用的验证、执行、错误处理流程
///
/// # 示例
/// ```ignore
/// let mut executor = ActionExecutor::new("add_item", cx);
/// executor.execute(
///     item,
///     |item| validation::validate_task_content(&item.content),
///     |item, store| Box::pin(add_item_with_store(item, store)),
///     |result, cx| { /* 成功回调 */ },
/// );
/// ```
pub struct ActionExecutor<'a> {
    operation_name: &'a str,
    cx: &'a mut App,
}

impl<'a> ActionExecutor<'a> {
    /// 创建新的动作执行器
    pub fn new(operation_name: &'a str, cx: &'a mut App) -> Self {
        Self { operation_name, cx }
    }

    /// 执行带验证的动作
    ///
    /// # 类型参数
    /// - `T`: 实体类型(ItemModel, ProjectModel 等)
    /// - `V`: 验证函数签名
    /// - `Op`: 异步操作函数
    /// - `S`: 成功回调
    ///
    /// # 参数
    /// - `entity`: 要操作的实体
    /// - `validator`: 验证函数
    /// - `operation`: 实际执行的异步操作
    /// - `on_success`: 成功回调
    pub fn execute<T, V, Op, S>(
        &mut self,
        entity: Arc<T>,
        validator: V,
        operation: Op,
        on_success: S,
    ) where
        T: Clone + std::fmt::Debug + 'static,
        V: FnOnce(&T) -> Result<(), AppError>,
        Op: FnOnce(
                Arc<T>,
                Arc<todos::Store>,
            ) -> Pin<Box<dyn Future<Output = Result<T, TodoError>> + Send>>
            + 'static,
        S: FnOnce(T, &mut AsyncApp) + 'static,
    {
        let entity_id = self.format_entity_id(&entity);

        if let Err(e) = validator(&entity) {
            let context = ErrorHandler::handle_with_location(e, self.operation_name);
            error!("{}", context.format_user_message());
            return;
        }

        let store = crate::core::state::get_store(self.cx);
        let op_name = self.operation_name.to_string();

        self.cx
            .spawn(async move |mut cx| match operation(entity.clone(), store).await {
                Ok(result) => {
                    info!("Successfully {} entity: {}", op_name, entity_id);
                    on_success(result, &mut cx);
                },
                Err(e) => {
                    let context = ErrorHandler::handle_with_resource(
                        AppError::Database(e),
                        &op_name,
                        &entity_id,
                    );
                    error!("{}", context.format_user_message());
                    cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                        notifier.set_error(context.format_user_message());
                    });
                },
            })
            .detach();
    }

    /// 执行不需要验证的动作
    pub fn execute_without_validation<T, Op, S>(
        &mut self,
        entity: Arc<T>,
        operation: Op,
        on_success: S,
    ) where
        T: Clone + std::fmt::Debug + 'static,
        Op: FnOnce(
                Arc<T>,
                Arc<todos::Store>,
            ) -> Pin<Box<dyn Future<Output = Result<T, TodoError>> + Send>>
            + 'static,
        S: FnOnce(T, &mut AsyncApp) + 'static,
    {
        let entity_id = self.format_entity_id(&entity);
        let store = crate::core::state::get_store(self.cx);
        let op_name = self.operation_name.to_string();

        self.cx
            .spawn(async move |mut cx| match operation(entity.clone(), store).await {
                Ok(result) => {
                    info!("Successfully {} entity: {}", op_name, entity_id);
                    on_success(result, &mut cx);
                },
                Err(e) => {
                    let context = ErrorHandler::handle_with_resource(
                        AppError::Database(e),
                        &op_name,
                        &entity_id,
                    );
                    error!("{}", context.format_user_message());
                    cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                        notifier.set_error(context.format_user_message());
                    });
                },
            })
            .detach();
    }

    /// 执行删除操作（返回空结果）
    pub fn execute_delete<T, Op, S>(&mut self, entity: Arc<T>, operation: Op, on_success: S)
    where
        T: Clone + std::fmt::Debug + 'static,
        Op: FnOnce(
                Arc<T>,
                Arc<todos::Store>,
            ) -> Pin<Box<dyn Future<Output = Result<(), TodoError>> + Send>>
            + 'static,
        S: FnOnce(&mut AsyncApp) + 'static,
    {
        let entity_id = self.format_entity_id(&entity);
        let store = crate::core::state::get_store(self.cx);
        let op_name = self.operation_name.to_string();

        self.cx
            .spawn(async move |mut cx| match operation(entity.clone(), store).await {
                Ok(()) => {
                    info!("Successfully {} entity: {}", op_name, entity_id);
                    on_success(&mut cx);
                },
                Err(e) => {
                    let context = ErrorHandler::handle_with_resource(
                        AppError::Database(e),
                        &op_name,
                        &entity_id,
                    );
                    error!("{}", context.format_user_message());
                    cx.update_global::<ErrorNotifier, _>(|notifier, _| {
                        notifier.set_error(context.format_user_message());
                    });
                },
            })
            .detach();
    }

    /// 格式化实体 ID 用于日志
    fn format_entity_id<T: std::fmt::Debug>(&self, entity: &T) -> String {
        format!("{:?}", entity)
    }
}

// ==================== 便捷函数 ====================

/// 创建任务内容的验证器
pub fn validate_task_content() -> impl FnOnce(&todos::entity::ItemModel) -> Result<(), AppError> {
    |item: &todos::entity::ItemModel| validation::validate_task_content(&item.content)
}

/// 创建项目名称的验证器
pub fn validate_project_name() -> impl FnOnce(&todos::entity::ProjectModel) -> Result<(), AppError>
{
    |project: &todos::entity::ProjectModel| validation::validate_task_content(&project.name)
}
