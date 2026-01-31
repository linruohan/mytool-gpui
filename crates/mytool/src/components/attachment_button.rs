use std::rc::Rc;

use gpui::{
    App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, StyleRefinement,
    Styled, Window, div, px,
};
use gpui_component::{
    IconName, Sizable, Size, StyleSized, StyledExt as _,
    button::{Button, ButtonVariants},
    h_flex, v_flex,
};
use sea_orm::prelude::Uuid;
use todos::entity::AttachmentModel;

use crate::todo_actions::delete_attachment;

pub enum AttachmentButtonEvent {
    Added(Rc<AttachmentModel>),
    Removed(String), // attachment id
}

pub struct AttachmentButtonState {
    focus_handle: FocusHandle,
    pub attachments: Vec<Rc<AttachmentModel>>,
    pub item_id: String,
}

impl EventEmitter<AttachmentButtonEvent> for AttachmentButtonState {}

impl Focusable for AttachmentButtonState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl AttachmentButtonState {
    pub fn new(item_id: String, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle(), attachments: Vec::new(), item_id }
    }

    pub fn set_attachments(
        &mut self,
        attachments: Vec<Rc<AttachmentModel>>,
        cx: &mut Context<Self>,
    ) {
        self.attachments = attachments;
        cx.notify();
    }

    pub fn add_attachment(&mut self, attachment: Rc<AttachmentModel>, cx: &mut Context<Self>) {
        self.attachments.push(attachment.clone());
        cx.emit(AttachmentButtonEvent::Added(attachment));
        cx.notify();
    }

    pub fn remove_attachment(&mut self, attachment_id: &str, cx: &mut Context<Self>) {
        self.attachments.retain(|a| a.id != attachment_id);
        cx.emit(AttachmentButtonEvent::Removed(attachment_id.to_string()));
        cx.notify();
    }

    fn on_add_attachment(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        // 使用 rfd 库打开文件选择框
        let item_id = self.item_id.clone();

        // 在后台线程中打开文件选择对话框
        std::thread::spawn(move || {
            // 打开文件选择对话框
            if let Some(file_path) = rfd::FileDialog::new().pick_file() {
                if let Some(file_name) = file_path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        let file_size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);

                        let file_type = file_path
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|s| s.to_string());

                        let attachment = AttachmentModel {
                            id: Uuid::new_v4().to_string(),
                            item_id: item_id.clone(),
                            file_name: file_name_str.to_string(),
                            file_path: file_path.to_string_lossy().to_string(),
                            file_type,
                            file_size,
                        };

                        // 注意：这里无法直接更新 UI，因为在后台线程中
                        // 需要通过全局状态或其他机制来通知 UI 更新
                        // 暂时只保存到数据库，UI 更新需要用户手动刷新或重新打开
                        println!("Selected file: {:?}", attachment.file_name);
                    }
                }
            }
        });
    }

    fn on_remove_attachment(&mut self, attachment_id: &str, cx: &mut Context<Self>) {
        self.remove_attachment(attachment_id, cx);

        // 从数据库删除
        delete_attachment(attachment_id.to_string(), cx);
    }
}

impl Render for AttachmentButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();

        v_flex()
            .gap_2()
            .child(
                h_flex().gap_2().items_center().child(
                    Button::new("add-attachment")
                        .small()
                        .outline()
                        .icon(IconName::MailAttachmentSymbolic)
                        .on_click({
                            let view = view.clone();
                            move |_event, window, cx| {
                                cx.update_entity(&view, |this, cx| {
                                    this.on_add_attachment(window, cx);
                                });
                            }
                        }),
                ),
            )
            .children(self.attachments.iter().enumerate().map(|(idx, attachment)| {
                let attachment_id = attachment.id.clone();
                let view = view.clone();

                h_flex()
                    .gap_2()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .py_1()
                    .border_1()
                    .rounded(px(4.0))
                    .child(
                        gpui_component::label::Label::new(attachment.file_name.clone()).text_sm(),
                    )
                    .child(
                        Button::new(format!("remove-attachment-{}", idx))
                            .small()
                            .ghost()
                            .compact()
                            .icon(IconName::UserTrashSymbolic)
                            .on_click({
                                let attachment_id = attachment_id.clone();
                                let view = view.clone();
                                move |_event, _window, cx| {
                                    cx.update_entity(&view, |this, cx| {
                                        this.on_remove_attachment(&attachment_id, cx);
                                    });
                                }
                            }),
                    )
            }))
    }
}

#[derive(IntoElement)]
pub struct AttachmentButton {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<AttachmentButtonState>,
}

impl Sizable for AttachmentButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for AttachmentButton {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for AttachmentButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl AttachmentButton {
    pub fn new(state: &Entity<AttachmentButtonState>) -> Self {
        Self {
            id: ("item-attachment", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for AttachmentButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .flex_none()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
