use std::path::{Path, PathBuf};

use gpui::App;
use todos::entity::AttachmentModel;

use crate::todo_state::{DBState, ErrorNotifier};

fn attachments_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("attachments")
}

fn safe_file_name(name: &str) -> String {
    name.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

/// 把用户选择的文件拷进应用目录，返回新路径。
pub fn copy_into_app_dir(
    src: &Path,
    item_id: &str,
    attachment_id: &str,
    original_name: &str,
) -> Result<PathBuf, String> {
    let dir = attachments_root().join(if item_id.is_empty() { "pending" } else { item_id });
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建附件目录失败: {e}"))?;
    let dest = dir.join(format!("{attachment_id}_{}", safe_file_name(original_name)));
    std::fs::copy(src, &dest).map_err(|e| format!("复制附件失败: {e}"))?;
    Ok(dest)
}

pub fn is_managed_attachment_path(path: &str) -> bool {
    let p = Path::new(path);
    p.components().any(|c| c.as_os_str() == "attachments")
}

pub fn remove_managed_file(path: &str) {
    if is_managed_attachment_path(path) {
        let _ = std::fs::remove_file(path);
    }
}

/// 用系统默认程序打开附件（异步，避免卡住 UI）。
pub fn open_attachment_path(path: &str, cx: &mut App) {
    if path.trim().is_empty() {
        return;
    }
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));
    cx.spawn(async move |_cx| {
        #[cfg(target_os = "windows")]
        {
            let _ = tokio::process::Command::new("cmd")
                .args(["/C", "start", "", &path.to_string_lossy()])
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = tokio::process::Command::new("open").arg(&path).spawn();
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            let _ = tokio::process::Command::new("xdg-open").arg(&path).spawn();
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_path_detects_attachments_dir() {
        assert!(is_managed_attachment_path(r"D:\app\attachments\abc\file.txt"));
        assert!(is_managed_attachment_path("/opt/mytool/attachments/abc/file.txt"));
        assert!(!is_managed_attachment_path(r"C:\Users\me\Desktop\photo.png"));
    }
}

fn notify_error(cx: &mut gpui::AsyncApp, message: String) {
    let _ = cx.update_global::<ErrorNotifier, _>(|notifier, _| {
        notifier.set_error(message);
    });
}

pub fn add_attachment(attachment: AttachmentModel, cx: &mut App) {
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(move |store| async move { store.insert_attachment(attachment).await })
            .await
        {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => {
                tracing::error!("add_attachment failed: {:?}", e);
                notify_error(cx, format!("添加附件失败：{e}"));
            },
            Err(join_err) => tracing::error!("add_attachment task panicked: {:?}", join_err),
        }
    })
    .detach();
}

pub fn delete_attachment(attachment_id: String, file_path: String, cx: &mut App) {
    remove_managed_file(&file_path);
    let db_state = cx.global::<DBState>().clone();
    cx.spawn(async move |cx| {
        match db_state
            .spawn_store_op(
                move |store| async move { store.delete_attachment(&attachment_id).await },
            )
            .await
        {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => {
                tracing::error!("delete_attachment failed: {:?}", e);
                notify_error(cx, format!("删除附件失败：{e}"));
            },
            Err(join_err) => tracing::error!("delete_attachment task panicked: {:?}", join_err),
        }
    })
    .detach();
}
