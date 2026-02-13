use std::sync::Arc;

use gpui::{Context, ParentElement, Render, Styled, Window, prelude::*};
use gpui_component::{
    WindowExt,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    v_flex,
};

use crate::{
    ItemInfoState,
    components::{ItemInfo, dialog::dialog::DialogConfig},
};

/// 统一的编辑对话框配置（Item / Section 公用）
#[derive(Clone)]
pub struct EditDialogConfig {
    pub title: String,
    pub button_label: String,
    pub is_edit: bool,
    pub overlay: bool,
}

/// 向后兼容的类型别名
pub type ItemDialogConfig = EditDialogConfig;
pub type SectionDialogConfig = EditDialogConfig;

impl EditDialogConfig {
    pub fn new(title: &str, button_label: &str, is_edit: bool) -> Self {
        Self {
            title: title.to_string(),
            button_label: button_label.to_string(),
            is_edit,
            overlay: true,
        }
    }

    pub fn with_overlay(mut self, overlay: bool) -> Self {
        self.overlay = overlay;
        self
    }

    /// 从旧的 ItemDialogConfig 语义构造（兼容优化文档）
    pub fn for_item(title: &str, button_label: &str, is_edit: bool) -> Self {
        Self::new(title, button_label, is_edit).with_overlay(true)
    }

    /// 从旧的 SectionDialogConfig 语义构造（兼容优化文档）
    pub fn for_section(title: &str, button_label: &str, is_edit: bool) -> Self {
        Self::new(title, button_label, is_edit).with_overlay(false)
    }
}

/// 通用编辑对话框入口，复用标题/按钮/overlay 等基础配置
fn show_edit_dialog<T, ContentFn, SaveFn>(
    window: &mut Window,
    cx: &mut Context<T>,
    config: EditDialogConfig,
    content_fn: ContentFn,
    save_fn: SaveFn,
) where
    T: Render + 'static,
    ContentFn: Fn() -> gpui::AnyElement + Clone + 'static,
    SaveFn: Fn(&mut gpui::App) + Clone + 'static,
{
    let dialog_config = DialogConfig::new(&config.title).overlay(config.overlay);

    window.open_dialog(cx, move |modal, _, _| {
        let dialog_config = dialog_config.clone();
        let config = config.clone();
        let content_fn = content_fn.clone();
        let save_fn = save_fn.clone();
        let cancel_label = dialog_config.cancel_label.clone();

        modal
            .title(dialog_config.title.clone())
            .overlay(dialog_config.overlay)
            .keyboard(dialog_config.keyboard)
            .overlay_closable(dialog_config.overlay_closable)
            .child((content_fn)())
            .footer(move |_, _, _, _| {
                let config = config.clone();
                let cancel_label = cancel_label.clone();
                let save_fn = save_fn.clone();

                vec![
                    Button::new("save").primary().label(&config.button_label).on_click(
                        move |_, window, cx| {
                            window.close_dialog(cx);
                            (save_fn)(cx);
                        },
                    ),
                    Button::new("cancel").label(&cancel_label).on_click(move |_, window, cx| {
                        window.close_dialog(cx);
                    }),
                ]
            })
    });
}

/// 显示 Item 编辑对话框
///
/// # 参数
/// - `window`: 窗口引用
/// - `cx`: 上下文
/// - `item_info`: ItemInfo 状态实体
/// - `config`: 对话框配置
/// - `on_save`: 保存回调，接收 item 数据
pub fn show_item_dialog<T, F>(
    window: &mut Window,
    cx: &mut Context<T>,
    item_info: gpui::Entity<ItemInfoState>,
    config: EditDialogConfig,
    on_save: F,
) where
    T: Render + 'static,
    F: Fn(Arc<todos::entity::ItemModel>, &mut gpui::App) + Clone + 'static,
{
    show_edit_dialog(
        window,
        cx,
        config,
        {
            let item_info = item_info.clone();
            move || {
                let info = ItemInfo::new(&item_info);
                info.into_any_element()
            }
        },
        {
            let item_info = item_info.clone();
            let on_save = on_save.clone();
            move |app_cx: &mut gpui::App| {
                let item = item_info.read(app_cx).item.clone();
                on_save(item, app_cx);
            }
        },
    );
}

/// 显示 Section 编辑对话框
///
/// # 参数
/// - `window`: 窗口引用
/// - `cx`: 上下文
/// - `name_input`: 输入框状态实体
/// - `config`: 对话框配置
/// - `on_save`: 保存回调，接收 section 名称
pub fn show_section_dialog<T, F>(
    window: &mut Window,
    cx: &mut Context<T>,
    name_input: gpui::Entity<InputState>,
    config: EditDialogConfig,
    on_save: F,
) where
    T: Render + 'static,
    F: Fn(String, &mut gpui::App) + Clone + 'static,
{
    show_edit_dialog(
        window,
        cx,
        config,
        {
            let name_input = name_input.clone();
            move || {
                let div = v_flex().gap_3().child(Input::new(&name_input));
                div.into_any_element()
            }
        },
        {
            let name_input = name_input.clone();
            let on_save = on_save.clone();
            move |app_cx: &mut gpui::App| {
                let name = name_input.read(app_cx).value().to_string();
                on_save(name, app_cx);
            }
        },
    );
}

/// 显示删除 Item 确认对话框
///
/// # 参数
/// - `window`: 窗口引用
/// - `cx`: 上下文
/// - `message`: 确认消息
/// - `on_ok`: 确认回调
pub fn show_delete_dialog<T, F>(window: &mut Window, cx: &mut Context<T>, message: &str, on_ok: F)
where
    T: Render + 'static,
    F: Fn(&mut gpui::App) + Clone + 'static,
{
    let message = message.to_string();
    let on_ok = on_ok.clone();

    window.open_dialog(cx, move |dialog, _, _| {
        let message = message.clone();
        let on_ok = on_ok.clone();

        dialog
            .confirm()
            .overlay(true)
            .overlay_closable(true)
            .child(message)
            .on_ok(move |_, window, cx| {
                on_ok(cx);
                window.push_notification("You have delete ok.", cx);
                true
            })
            .on_cancel(|_, window, cx| {
                window.push_notification("You have canceled delete.", cx);
                true
            })
    });
}

/// 显示删除 Item 确认对话框（兼容旧接口）
pub fn show_item_delete_dialog<T, F>(
    window: &mut Window,
    cx: &mut Context<T>,
    message: &str,
    on_ok: F,
) where
    T: Render + 'static,
    F: Fn(&mut gpui::App) + Clone + 'static,
{
    show_delete_dialog(window, cx, message, on_ok);
}

/// 显示删除 Section 确认对话框（兼容旧接口）
pub fn show_section_delete_dialog<T, F>(
    window: &mut Window,
    cx: &mut Context<T>,
    message: &str,
    on_ok: F,
) where
    T: Render + 'static,
    F: Fn(&mut gpui::App) + Clone + 'static,
{
    show_delete_dialog(window, cx, message, on_ok);
}
