/// 通知系统使用示例
/// 
/// 这个文件展示了如何在不同场景中使用新的通知系统

use std::sync::Arc;
use gpui::{Context, Window};
use crate::core::notification::NotificationSystem;

// ============================================================================
// 示例 1: 基本的 CRUD 操作反馈
// ============================================================================

struct ItemManager;

impl ItemManager {
    /// 创建新项目
    fn create_item(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        NotificationSystem::debug(format!("Creating item: {}", name));
        
        match self.validate_name(&name) {
            Ok(_) => {
                // 执行创建操作
                self.do_create(name);
                NotificationSystem::success("Item created successfully", window, cx);
            }
            Err(msg) => {
                NotificationSystem::warning(msg, window, cx);
            }
        }
    }
    
    /// 更新项目
    fn update_item(&mut self, id: String, name: String, window: &mut Window, cx: &mut Context<Self>) {
        NotificationSystem::debug(format!("Updating item {}", id));
        
        match self.do_update(id, name) {
            Ok(_) => {
                NotificationSystem::success("Item updated", window, cx);
            }
            Err(e) => {
                NotificationSystem::error(
                    format!("Failed to update item: {}", e),
                    window,
                    cx
                );
            }
        }
    }
    
    /// 删除项目（带确认）
    fn delete_item(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        // 显示确认对话框
        Dialog::new("Delete Item")
            .message("This action cannot be undone.")
            .on_confirm(move |this, window, cx| {
                match this.do_delete(id.clone()) {
                    Ok(_) => {
                        NotificationSystem::success("Item deleted", window, cx);
                    }
                    Err(e) => {
                        NotificationSystem::error(
                            format!("Failed to delete: {}", e),
                            window,
                            cx
                        );
                    }
                }
                true
            })
            .on_cancel(|_, window, cx| {
                NotificationSystem::info("Delete cancelled", window, cx);
                true
            })
            .show(window, cx);
    }
    
    // 辅助方法
    fn validate_name(&self, name: &str) -> Result<(), String> {
        if name.is_empty() {
            Err("Item name cannot be empty".to_string())
        } else {
            Ok(())
        }
    }
    
    fn do_create(&mut self, _name: String) {
        // 实际创建逻辑
    }
    
    fn do_update(&mut self, _id: String, _name: String) -> Result<(), String> {
        // 实际更新逻辑
        Ok(())
    }
    
    fn do_delete(&mut self, _id: String) -> Result<(), String> {
        // 实际删除逻辑
        Ok(())
    }
}

// ============================================================================
// 示例 2: 异步数据加载
// ============================================================================

struct DataLoader;

impl DataLoader {
    /// 加载数据（带进度提示）
    fn load_data(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        NotificationSystem::info("Loading data...", window, cx);
        
        let db = get_db_connection(cx);
        
        cx.spawn(async move |this, mut cx| {
            NotificationSystem::debug("Starting data fetch");
            
            match fetch_data_from_db(db).await {
                Ok(data) => {
                    NotificationSystem::debug(format!("Loaded {} items", data.len()));
                    
                    // 更新 UI
                    cx.update_entity(&this, |state, cx| {
                        state.set_data(data);
                        cx.notify();
                    });
                    
                    // 显示成功通知
                    cx.update(|cx| {
                        if let Some(window) = cx.windows().first() {
                            NotificationSystem::success("Data loaded", window, cx);
                        }
                    }).ok();
                }
                Err(e) => {
                    NotificationSystem::log_error("Failed to load data", &e);
                    
                    // 显示错误通知
                    cx.update(|cx| {
                        if let Some(window) = cx.windows().first() {
                            NotificationSystem::error(
                                "Failed to load data. Please try again.",
                                window,
                                cx
                            );
                        }
                    }).ok();
                }
            }
        }).detach();
    }
    
    /// 后台同步（静默）
    fn background_sync(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, _cx| {
            NotificationSystem::debug("Starting background sync");
            
            match sync_with_server().await {
                Ok(count) => {
                    NotificationSystem::debug(format!("Synced {} items", count));
                }
                Err(e) => {
                    // 后台操作失败，只记录日志，不打扰用户
                    NotificationSystem::log_error("Background sync failed", &e);
                }
            }
        }).detach();
    }
    
    fn set_data(&mut self, _data: Vec<String>) {
        // 设置数据
    }
}

// ============================================================================
// 示例 3: 表单验证
// ============================================================================

struct FormValidator;

impl FormValidator {
    /// 验证并提交表单
    fn submit_form(&mut self, form_data: FormData, window: &mut Window, cx: &mut Context<Self>) {
        // 验证表单
        let validation_errors = self.validate(&form_data);
        
        if !validation_errors.is_empty() {
            // 显示验证错误
            let error_msg = validation_errors.join(", ");
            NotificationSystem::warning(
                format!("Please fix: {}", error_msg),
                window,
                cx
            );
            return;
        }
        
        // 提交表单
        NotificationSystem::info("Submitting...", window, cx);
        
        match self.do_submit(form_data) {
            Ok(_) => {
                NotificationSystem::success("Form submitted successfully", window, cx);
            }
            Err(e) => {
                NotificationSystem::error(
                    format!("Submission failed: {}", e),
                    window,
                    cx
                );
            }
        }
    }
    
    fn validate(&self, data: &FormData) -> Vec<String> {
        let mut errors = Vec::new();
        
        if data.name.is_empty() {
            errors.push("Name is required".to_string());
        }
        
        if data.email.is_empty() {
            errors.push("Email is required".to_string());
        } else if !data.email.contains('@') {
            errors.push("Invalid email format".to_string());
        }
        
        errors
    }
    
    fn do_submit(&mut self, _data: FormData) -> Result<(), String> {
        // 实际提交逻辑
        Ok(())
    }
}

// ============================================================================
// 示例 4: 批量操作
// ============================================================================

struct BatchProcessor;

impl BatchProcessor {
    /// 批量处理项目
    fn process_batch(&mut self, items: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        let total = items.len();
        NotificationSystem::info(
            format!("Processing {} items...", total),
            window,
            cx
        );
        
        let db = get_db_connection(cx);
        
        cx.spawn(async move |_this, mut cx| {
            let mut success_count = 0;
            let mut error_count = 0;
            
            for (index, item) in items.iter().enumerate() {
                NotificationSystem::debug(format!("Processing item {}/{}", index + 1, total));
                
                match process_item(item, &db).await {
                    Ok(_) => success_count += 1,
                    Err(e) => {
                        error_count += 1;
                        NotificationSystem::log_error(
                            format!("Failed to process item {}", item),
                            &e
                        );
                    }
                }
            }
            
            // 显示最终结果
            cx.update(|cx| {
                if let Some(window) = cx.windows().first() {
                    if error_count == 0 {
                        NotificationSystem::success(
                            format!("All {} items processed successfully", success_count),
                            window,
                            cx
                        );
                    } else if success_count == 0 {
                        NotificationSystem::error(
                            format!("Failed to process all {} items", error_count),
                            window,
                            cx
                        );
                    } else {
                        NotificationSystem::warning(
                            format!("Processed {} items, {} failed", success_count, error_count),
                            window,
                            cx
                        );
                    }
                }
            }).ok();
        }).detach();
    }
}

// ============================================================================
// 示例 5: 网络操作
// ============================================================================

struct NetworkManager;

impl NetworkManager {
    /// 上传文件
    fn upload_file(&mut self, file_path: String, window: &mut Window, cx: &mut Context<Self>) {
        NotificationSystem::info("Uploading file...", window, cx);
        
        cx.spawn(async move |_this, mut cx| {
            match upload_to_server(&file_path).await {
                Ok(url) => {
                    NotificationSystem::debug(format!("File uploaded to: {}", url));
                    
                    cx.update(|cx| {
                        if let Some(window) = cx.windows().first() {
                            NotificationSystem::success("File uploaded successfully", window, cx);
                        }
                    }).ok();
                }
                Err(e) => {
                    NotificationSystem::log_error("Upload failed", &e);
                    
                    cx.update(|cx| {
                        if let Some(window) = cx.windows().first() {
                            NotificationSystem::error(
                                "Upload failed. Please check your connection.",
                                window,
                                cx
                            );
                        }
                    }).ok();
                }
            }
        }).detach();
    }
    
    /// 检查网络连接
    fn check_connection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, mut cx| {
            NotificationSystem::debug("Checking network connection");
            
            match ping_server().await {
                Ok(latency) => {
                    if latency > 1000 {
                        cx.update(|cx| {
                            if let Some(window) = cx.windows().first() {
                                NotificationSystem::warning(
                                    "Network connection is slow",
                                    window,
                                    cx
                                );
                            }
                        }).ok();
                    } else {
                        NotificationSystem::debug(format!("Connection OK ({}ms)", latency));
                    }
                }
                Err(e) => {
                    NotificationSystem::log_error("Connection check failed", &e);
                    
                    cx.update(|cx| {
                        if let Some(window) = cx.windows().first() {
                            NotificationSystem::error(
                                "No network connection",
                                window,
                                cx
                            );
                        }
                    }).ok();
                }
            }
        }).detach();
    }
}

// ============================================================================
// 辅助类型和函数（示例用）
// ============================================================================

struct FormData {
    name: String,
    email: String,
}

struct Dialog;
impl Dialog {
    fn new(_title: &str) -> Self { Self }
    fn message(self, _msg: &str) -> Self { self }
    fn on_confirm<F>(self, _f: F) -> Self where F: Fn(&mut ItemManager, &mut Window, &mut Context<ItemManager>) -> bool { self }
    fn on_cancel<F>(self, _f: F) -> Self where F: Fn(&mut ItemManager, &mut Window, &mut Context<ItemManager>) -> bool { self }
    fn show(self, _window: &mut Window, _cx: &mut Context<ItemManager>) {}
}

fn get_db_connection<T>(_cx: &mut Context<T>) -> Arc<()> { Arc::new(()) }
async fn fetch_data_from_db(_db: Arc<()>) -> Result<Vec<String>, String> { Ok(vec![]) }
async fn sync_with_server() -> Result<usize, String> { Ok(0) }
async fn process_item(_item: &str, _db: &Arc<()>) -> Result<(), String> { Ok(()) }
async fn upload_to_server(_path: &str) -> Result<String, String> { Ok("".to_string()) }
async fn ping_server() -> Result<u64, String> { Ok(100) }
