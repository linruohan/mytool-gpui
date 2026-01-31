use std::rc::Rc;

use gpui::{
    App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement as _, Render, RenderOnce, StyleRefinement,
    Styled, Subscription, Window, div,
};
use gpui_component::{
    IconName, Sizable, Size, StyleSized, StyledExt as _,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    popover::Popover,
    v_flex,
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
    popover_open: bool,
    search_input: Entity<InputState>,
    search_query: String,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<AttachmentButtonEvent> for AttachmentButtonState {}

impl Focusable for AttachmentButtonState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl AttachmentButtonState {
    pub fn new(item_id: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search attachments..."));
        let _subscriptions = vec![cx.subscribe_in(&search_input, window, Self::on_search_event)];
        Self {
            focus_handle: cx.focus_handle(),
            attachments: Vec::new(),
            item_id,
            popover_open: false,
            search_input,
            search_query: String::new(),
            _subscriptions,
        }
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

    fn on_search_event(
        &mut self,
        _state: &Entity<InputState>,
        event: &InputEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::Change = event {
            let query = self.search_input.read(cx).value().to_string();
            self.search_query = query;
            cx.notify();
        }
    }

    fn get_filtered_attachments(&self) -> Vec<Rc<AttachmentModel>> {
        if self.search_query.is_empty() {
            self.attachments.clone()
        } else {
            self.attachments
                .iter()
                .filter(|a| a.file_name.to_lowercase().contains(&self.search_query.to_lowercase()))
                .cloned()
                .collect()
        }
    }

    fn on_add_attachment(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let item_id = self.item_id.clone();

        // 直接打开文件选择对话框（这是同步的）
        if let Some(file_path) = rfd::FileDialog::new().pick_file()
            && let Some(file_name) = file_path.file_name()
            && let Some(file_name_str) = file_name.to_str()
        {
            let file_size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);

            let file_type =
                file_path.extension().and_then(|ext| ext.to_str()).map(|s| s.to_string());

            let attachment = AttachmentModel {
                id: Uuid::new_v4().to_string(),
                item_id: item_id.clone(),
                file_name: file_name_str.to_string(),
                file_path: file_path.to_string_lossy().to_string(),
                file_type,
                file_size,
            };

            // 添加到列表并保存到数据库
            self.add_attachment(Rc::new(attachment.clone()), cx);
            crate::todo_actions::add_attachment(attachment, cx);
        }
    }

    fn on_remove_attachment(&mut self, attachment_id: &str, cx: &mut Context<Self>) {
        self.remove_attachment(attachment_id, cx);
        delete_attachment(attachment_id.to_string(), cx);
    }
}

impl Render for AttachmentButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        let popover_open = self.popover_open;
        let search_input = self.search_input.clone();
        let filtered_attachments = self.get_filtered_attachments();

        Popover::new("attachment-popover")
            .p_0()
            .text_sm()
            .open(popover_open)
            .on_open_change(cx.listener(move |this, open, _, cx| {
                this.popover_open = *open;
                if !*open {
                    this.search_query.clear();
                }
                cx.notify();
            }))
            .trigger(
                Button::new("open-attachment-dialog")
                    .small()
                    .outline()
                    .icon(IconName::MailAttachmentSymbolic),
            )
            .track_focus(&self.focus_handle)
            .child(
                v_flex()
                    .gap_3()
                    .p_3()
                    .w_96()
                    // 搜索框和添加按钮
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Input::new(&search_input)
                                    .flex_1(),
                            )
                            .child(
                                Button::new("add-attachment-dialog")
                                    .small()
                                    .primary()
                                    .icon(IconName::Plus)
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
                    // 附件列表
                    .child(
                        v_flex()
                            .gap_2()
                            .children(filtered_attachments.iter().enumerate().map(
                                |(idx, attachment)| {
                                    let attachment_id = attachment.id.clone();
                                    let view = view.clone();

                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .justify_between()
                                        .px_2()
                                        .py_2()
                                        .border_b_1()
                                        .child(
                                            gpui_component::label::Label::new(
                                                attachment.file_name.clone(),
                                            )
                                                .text_sm(),
                                        )
                                        .child(
                                            Button::new(format!(
                                                "remove-attachment-dialog-{}",
                                                idx
                                            ))
                                                .small()
                                                .ghost()
                                                .compact()
                                                .icon(IconName::UserTrashSymbolic)
                                                .on_click({
                                                    let attachment_id = attachment_id.clone();
                                                    let view = view.clone();
                                                    move |_event, _window, cx| {
                                                        cx.update_entity(&view, |this, cx| {
                                                            this.on_remove_attachment(
                                                                &attachment_id,
                                                                cx,
                                                            );
                                                        });
                                                    }
                                                }),
                                        )
                                },
                            )),
                    ),
            )
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
