use std::rc::Rc;

use gpui::{Context, ParentElement, Render, Styled, Window};
use gpui_component::{
    WindowExt,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    v_flex,
};

use crate::{
    ItemInfoState,
    components::{DialogConfig, ItemInfo},
};

/// Item Dialog 配置
#[derive(Clone)]
pub struct ItemDialogConfig {
    pub title: String,
    pub button_label: String,
    pub is_edit: bool,
}

impl ItemDialogConfig {
    pub fn new(title: &str, button_label: &str, is_edit: bool) -> Self {
        Self { title: title.to_string(), button_label: button_label.to_string(), is_edit }
    }
}

/// Section Dialog 配置
#[derive(Clone)]
pub struct SectionDialogConfig {
    pub title: String,
    pub button_label: String,
    pub is_edit: bool,
    pub overlay: bool,
}

impl SectionDialogConfig {
    pub fn new(title: &str, button_label: &str, is_edit: bool) -> Self {
        Self {
            title: title.to_string(),
            button_label: button_label.to_string(),
            is_edit,
            overlay: false,
        }
    }

    pub fn with_overlay(mut self, overlay: bool) -> Self {
        self.overlay = overlay;
        self
    }
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
    config: ItemDialogConfig,
    on_save: F,
) where
    T: Render + 'static,
    F: Fn(Rc<todos::entity::ItemModel>, &mut gpui::App) + Clone + 'static,
{
    let dialog_config = DialogConfig::new(&config.title);

    window.open_dialog(cx, move |modal, _, _| {
        let item_info = item_info.clone();
        let config = config.clone();
        let cancel_label = dialog_config.cancel_label.clone();
        let on_save = on_save.clone();

        modal
            .title(dialog_config.title.clone())
            .overlay(dialog_config.overlay)
            .keyboard(dialog_config.keyboard)
            .overlay_closable(dialog_config.overlay_closable)
            .child(ItemInfo::new(&item_info))
            .footer(move |_, _, _, _| {
                let item_info = item_info.clone();
                let config = config.clone();
                let cancel_label = cancel_label.clone();
                let on_save = on_save.clone();

                vec![
                    Button::new("save").primary().label(&config.button_label).on_click(
                        move |_, window, cx| {
                            window.close_dialog(cx);
                            let item = item_info.read(cx).item.clone();
                            on_save(item, cx);
                        },
                    ),
                    Button::new("cancel").label(&cancel_label).on_click(move |_, window, cx| {
                        window.close_dialog(cx);
                    }),
                ]
            })
    });
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
    config: SectionDialogConfig,
    on_save: F,
) where
    T: Render + 'static,
    F: Fn(String, &mut gpui::App) + Clone + 'static,
{
    let dialog_config = DialogConfig::new(&config.title).overlay(config.overlay);

    window.open_dialog(cx, move |modal, _, _| {
        let name_input = name_input.clone();
        let config = config.clone();
        let cancel_label = dialog_config.cancel_label.clone();
        let on_save = on_save.clone();

        modal
            .title(dialog_config.title.clone())
            .overlay(dialog_config.overlay)
            .keyboard(dialog_config.keyboard)
            .overlay_closable(dialog_config.overlay_closable)
            .child(v_flex().gap_3().child(Input::new(&name_input)))
            .footer(move |_, _, _, _| {
                let name_input = name_input.clone();
                let config = config.clone();
                let cancel_label = cancel_label.clone();
                let on_save = on_save.clone();

                vec![
                    Button::new("save").primary().label(&config.button_label).on_click(
                        move |_, window, cx| {
                            window.close_dialog(cx);
                            let name = name_input.read(cx).value().to_string();
                            on_save(name, cx);
                        },
                    ),
                    Button::new("cancel").label(&cancel_label).on_click(move |_, window, cx| {
                        window.close_dialog(cx);
                    }),
                ]
            })
    });
}
