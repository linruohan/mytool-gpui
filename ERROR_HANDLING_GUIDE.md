# 错误处理使用指南

## 概述

MyTool 提供了统一的错误处理系统，确保所有错误都能被正确处理、记录和向用户展示。

## 错误类型

### AppError 枚举

```rust
pub enum AppError {
    Database(TodoError),      // 数据库错误
    Validation(String),       // 验证错误
    Permission(String),       // 权限错误
    NotFound(String),         // 资源未找到
    Network(String),          // 网络错误
    FileSystem(IoError),      // 文件系统错误
    Config(String),           // 配置错误
    Parse(String),            // 解析错误
    Concurrency(String),      // 并发错误
    Internal(String),         // 内部错误
    Cancelled,                // 用户取消
    Timeout(String),          // 超时错误
    Other(String),            // 其他错误
}
```

## 错误严重程度

```rust
pub enum ErrorSeverity {
    Info,      // 信息：不影响功能
    Warning,   // 警告：可能影响功能
    Error,     // 错误：影响功能
    Critical,  // 严重：严重影响功能
}
```

## 使用方法

### 1. 基本错误处理

```rust
use crate::error_handler::{AppError, ErrorHandler, AppResult};

// 返回 Result
fn add_task(content: &str) -> AppResult<ItemModel> {
    // 验证输入
    if content.is_empty() {
        return Err(AppError::Validation("任务内容不能为空".to_string()));
    }
    
    // 执行操作
    let task = create_task(content)?;
    Ok(task)
}

// 处理错误
match add_task("新任务") {
    Ok(task) => {
        // 成功处理
    }
    Err(e) => {
        // 统一错误处理
        let context = ErrorHandler::handle(e);
        // 显示用户友好的错误消息
        show_error_message(&context.format_user_message());
    }
}
```

### 2. 带位置信息的错误处理

```rust
fn update_task(task_id: &str, content: &str) -> AppResult<()> {
    // ... 操作
    
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            // 记录错误发生的位置
            let context = ErrorHandler::handle_with_location(
                e,
                "update_task"
            );
            Err(AppError::from(context.error))
        }
    }
}
```

### 3. 带资源 ID 的错误处理

```rust
fn delete_task(task_id: &str) -> AppResult<()> {
    match db.delete(task_id) {
        Ok(_) => Ok(()),
        Err(e) => {
            // 记录错误和相关资源
            let context = ErrorHandler::handle_with_resource(
                AppError::Database(e),
                "delete_task",
                task_id
            );
            Err(AppError::from(context.error))
        }
    }
}
```

### 4. 输入验证

```rust
use crate::error_handler::validation;

// 验证任务内容
validation::validate_task_content(content)?;

// 验证项目名称
validation::validate_project_name(name)?;

// 验证标签名称
validation::validate_label_name(label)?;

// 清理 HTML 内容
let safe_content = validation::sanitize_html(user_input);
```

## 错误上下文

### ErrorContext 结构

```rust
pub struct ErrorContext {
    pub error: String,                      // 错误类型
    pub severity: ErrorSeverity,            // 严重程度
    pub user_message: String,               // 用户友好消息
    pub technical_details: String,          // 技术详情
    pub recovery_suggestions: Vec<String>,  // 恢复建议
    pub location: Option<String>,           // 错误位置
    pub resource_id: Option<String>,        // 资源 ID
}
```

### 使用示例

```rust
let context = ErrorContext::new(error)
    .with_location("add_task")
    .with_resource_id(task_id);

// 记录日志
context.log();

// 生成用户消息
let message = context.format_user_message();
// 输出：
// ❌ 数据库操作失败，请稍后重试
//
// 建议：
// 1. 检查数据库文件是否存在
// 2. 尝试重启应用
// 3. 如果问题持续，请联系技术支持
```

## 实际应用示例

### 示例 1: 添加任务

```rust
pub fn add_item(item: Arc<ItemModel>, cx: &mut App) {
    // 验证输入
    if let Err(e) = validation::validate_task_content(&item.content) {
        let context = ErrorHandler::handle_with_location(e, "add_item");
        show_error_toast(&context.format_user_message(), cx);
        return;
    }
    
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::add_item(item.clone(), db).await {
            Ok(new_item) => {
                cx.update_global::<TodoStore, _>(|store, _| {
                    store.add_item(Arc::new(new_item));
                });
            }
            Err(e) => {
                let context = ErrorHandler::handle_with_resource(
                    AppError::Database(e),
                    "add_item",
                    &item.id
                );
                show_error_toast(&context.format_user_message(), cx);
            }
        }
    }).detach();
}
```

### 示例 2: 批量操作错误处理

```rust
pub fn batch_add_items(items: Vec<Arc<ItemModel>>, cx: &mut App) {
    // 验证所有输入
    for item in &items {
        if let Err(e) = validation::validate_task_content(&item.content) {
            let context = ErrorHandler::handle_with_resource(
                e,
                "batch_add_items",
                &item.id
            );
            show_error_toast(&context.format_user_message(), cx);
            return;
        }
    }
    
    let db = cx.global::<DBState>().conn.clone();
    cx.spawn(async move |cx| {
        match crate::state_service::batch_add_items(items_vec, db).await {
            Ok(new_items) => {
                info!("Successfully added {} items", new_items.len());
                // 更新状态...
            }
            Err(e) => {
                let context = ErrorHandler::handle_with_location(
                    AppError::Database(e),
                    "batch_add_items"
                );
                show_error_toast(&context.format_user_message(), cx);
            }
        }
    }).detach();
}
```

### 示例 3: 文件操作错误处理

```rust
pub fn import_tasks_from_file(file_path: &str, cx: &mut App) {
    cx.spawn(async move |cx| {
        // 读取文件
        let content = match tokio::fs::read_to_string(file_path).await {
            Ok(c) => c,
            Err(e) => {
                let context = ErrorHandler::handle_with_location(
                    AppError::FileSystem(e),
                    "import_tasks_from_file"
                );
                show_error_toast(&context.format_user_message(), cx);
                return;
            }
        };
        
        // 解析 JSON
        let tasks: Vec<ItemModel> = match serde_json::from_str(&content) {
            Ok(t) => t,
            Err(e) => {
                let context = ErrorHandler::handle_with_location(
                    AppError::Parse(e.to_string()),
                    "import_tasks_from_file"
                );
                show_error_toast(&context.format_user_message(), cx);
                return;
            }
        };
        
        // 批量导入
        batch_add_items(tasks.into_iter().map(Arc::new).collect(), cx);
    }).detach();
}
```

## 错误日志

### 日志级别

```rust
// Info: 信息性错误（如用户取消操作）
context.severity = ErrorSeverity::Info;
// 输出: INFO location=add_task resource_id=task_123 ...

// Warning: 警告性错误（如验证失败）
context.severity = ErrorSeverity::Warning;
// 输出: WARN location=add_task resource_id=task_123 ...

// Error/Critical: 严重错误
context.severity = ErrorSeverity::Error;
// 输出: ERROR location=add_task resource_id=task_123 severity=Error ...
```

### 查看日志

```bash
# 查看所有错误日志
grep "ERROR" logs/app.log

# 查看特定位置的错误
grep "location=add_task" logs/app.log

# 查看特定资源的错误
grep "resource_id=task_123" logs/app.log
```

## 用户界面集成

### 错误提示组件（待实现）

```rust
// Toast 提示
fn show_error_toast(message: &str, cx: &mut App) {
    // 显示临时提示
    cx.show_toast(message, ToastType::Error);
}

// 对话框
fn show_error_dialog(context: &ErrorContext, cx: &mut App) {
    // 显示详细错误对话框
    cx.show_dialog(ErrorDialog::new(context));
}

// 内联错误
fn show_inline_error(message: &str, element_id: &str, cx: &mut App) {
    // 在表单元素旁显示错误
    cx.set_element_error(element_id, message);
}
```

## 最佳实践

### 1. 始终使用 AppResult

```rust
// ✅ 好的做法
fn add_task(content: &str) -> AppResult<ItemModel> {
    // ...
}

// ❌ 避免
fn add_task(content: &str) -> Result<ItemModel, String> {
    // ...
}
```

### 2. 提供有用的错误消息

```rust
// ✅ 好的做法
Err(AppError::Validation("任务内容不能为空".to_string()))

// ❌ 避免
Err(AppError::Validation("invalid input".to_string()))
```

### 3. 记录错误上下文

```rust
// ✅ 好的做法
let context = ErrorHandler::handle_with_resource(
    error,
    "add_task",
    &task_id
);

// ❌ 避免
error!("Error: {:?}", error);
```

### 4. 不要吞掉错误

```rust
// ✅ 好的做法
match operation() {
    Ok(result) => handle_success(result),
    Err(e) => {
        let context = ErrorHandler::handle(e);
        show_error(&context);
    }
}

// ❌ 避免
match operation() {
    Ok(result) => handle_success(result),
    Err(_) => {
        // 忽略错误
    }
}
```

### 5. 使用验证函数

```rust
// ✅ 好的做法
validation::validate_task_content(content)?;
let safe_content = validation::sanitize_html(content);

// ❌ 避免
if content.is_empty() {
    return Err(AppError::Other("empty".to_string()));
}
```

## 错误恢复策略

### 1. 自动重试

```rust
async fn add_task_with_retry(task: ItemModel, max_retries: usize) -> AppResult<ItemModel> {
    let mut attempts = 0;
    
    loop {
        match add_task_internal(&task).await {
            Ok(result) => return Ok(result),
            Err(e) if attempts < max_retries => {
                attempts += 1;
                warn!("Retry attempt {} after error: {:?}", attempts, e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 2. 降级处理

```rust
async fn get_tasks() -> AppResult<Vec<ItemModel>> {
    // 尝试从数据库加载
    match load_from_database().await {
        Ok(tasks) => Ok(tasks),
        Err(e) => {
            warn!("Database failed, using cache: {:?}", e);
            // 降级到缓存
            load_from_cache()
        }
    }
}
```

### 3. 用户确认

```rust
fn delete_task_with_confirmation(task_id: &str, cx: &mut App) -> AppResult<()> {
    // 显示确认对话框
    if !show_confirmation_dialog("确定要删除此任务吗？", cx) {
        return Err(AppError::Cancelled);
    }
    
    delete_task(task_id)
}
```

## 测试

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error() {
        let result = validation::validate_task_content("");
        assert!(result.is_err());
        
        if let Err(AppError::Validation(msg)) = result {
            assert!(msg.contains("不能为空"));
        }
    }

    #[test]
    fn test_error_context() {
        let error = AppError::NotFound("task_123".to_string());
        let context = ErrorContext::new(error);
        
        assert_eq!(context.severity, ErrorSeverity::Warning);
        assert!(!context.recovery_suggestions.is_empty());
    }
}
```

## 总结

统一的错误处理系统提供了：

- ✅ 一致的错误类型和处理方式
- ✅ 用户友好的错误消息
- ✅ 详细的错误日志
- ✅ 错误恢复建议
- ✅ 输入验证和清理
- ✅ 易于测试和维护

遵循本指南可以确保应用的错误处理既专业又用户友好。
