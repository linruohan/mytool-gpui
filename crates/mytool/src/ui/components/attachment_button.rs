use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, ParentElement, Render,
    Styled, Window,
};
use gpui_component::{
    IconName, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    popover::Popover,
    v_flex,
};
use sea_orm::prelude::Uuid;
use todos::entity::AttachmentModel;

use crate::{
    create_button_wrapper,
    todo_actions::delete_attachment,
    ui::components::{
        PopoverListMixin, PopoverSearchMixin, create_list_item_element, handle_search_input_change,
        manage_popover_state,
    },
};

pub type AttachmentResult<T> = Result<T, AttachmentError>;

#[derive(Debug, Clone)]
pub enum AttachmentError {
    FileNotFound(String),
    InvalidFileName,
    FileReadError(String),
}

impl std::fmt::Display for AttachmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound(path) => write!(f, "File not found: {}", path),
            Self::InvalidFileName => write!(f, "Invalid file name"),
            Self::FileReadError(msg) => write!(f, "File read error: {}", msg),
        }
    }
}

impl std::error::Error for AttachmentError {}

#[derive(Debug)]
pub enum AttachmentButtonEvent {
    Added(Arc<AttachmentModel>),
    Removed(String),
    Error(Box<dyn std::error::Error + Send + Sync>),
}

pub struct AttachmentButtonState {
    focus_handle: FocusHandle,
    pub item_id: String,
    search: PopoverSearchMixin,
    items: PopoverListMixin<Arc<AttachmentModel>>,
    /// 待保存的附件列表（当 item_id 从临时 ID 变为真实 ID 后保存）
    pending_attachments: Vec<AttachmentModel>,
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

        // Subscribe to search events directly
        let _ = cx.subscribe_in(&search_input, window, Self::on_search_event);

        let filter_fn = |attachment: &Arc<AttachmentModel>, query: &str| {
            attachment.file_name.to_lowercase().contains(&query.to_lowercase())
        };

        Self {
            focus_handle: cx.focus_handle(),
            item_id,
            search: PopoverSearchMixin::new(search_input),
            items: PopoverListMixin::new(filter_fn),
            pending_attachments: Vec::new(),
        }
    }

    pub fn set_attachments(
        &mut self,
        attachments: Vec<Arc<AttachmentModel>>,
        cx: &mut Context<Self>,
    ) {
        self.items.set_items(attachments);
        cx.notify();
    }

    /// 更新 item_id（用于临时ID变为真实ID时）
    pub fn update_item_id(&mut self, new_item_id: String, cx: &mut Context<Self>) {
        if self.item_id != new_item_id {
            let old_id = self.item_id.clone();
            tracing::info!(
                "AttachmentButtonState: updating item_id from {} to {}",
                old_id,
                new_item_id
            );
            self.item_id = new_item_id.clone();

            // 如果有待保存的附件，现在保存它们
            if !self.pending_attachments.is_empty() {
                tracing::info!(
                    "Saving {} pending attachments with new item_id: {}",
                    self.pending_attachments.len(),
                    new_item_id
                );

                // 取出待保存的附件
                let pending = std::mem::take(&mut self.pending_attachments);

                // 更新每个附件的 item_id 并保存
                for mut attachment in pending {
                    attachment.item_id = new_item_id.clone();
                    crate::todo_actions::add_attachment(attachment, cx);
                }
            }

            cx.notify();
        }
    }

    pub fn add_attachment(&mut self, attachment: Arc<AttachmentModel>, cx: &mut Context<Self>) {
        self.items.add_item(attachment.clone());
        cx.emit(AttachmentButtonEvent::Added(attachment));
        cx.notify();
    }

    pub fn remove_attachment(&mut self, attachment_id: &str, cx: &mut Context<Self>) {
        self.items.remove_item(|a| a.id == attachment_id);
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
            handle_search_input_change(
                &self.search.search_input,
                &mut self.search.search_query,
                cx,
            );
        }
    }

    fn get_filtered_attachments(&self) -> Vec<Arc<AttachmentModel>> {
        self.items.get_filtered(&self.search.search_query)
    }

    fn on_add_attachment(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.try_add_attachment(cx);
    }

    fn try_add_attachment(&mut self, cx: &mut Context<Self>) {
        let item_id = self.item_id.clone();
        let is_temp_id = item_id.starts_with("temp_");
        let view = cx.entity();

        cx.spawn(async move |_this, cx| {
            let file_handle = rfd::AsyncFileDialog::new().pick_file().await;
            let file_handle = match file_handle {
                Some(handle) => handle,
                None => return, // User cancelled
            };

            let file_path = file_handle.path().to_path_buf();
            let file_name = file_handle.file_name();
            let file_size = match std::fs::metadata(&file_path) {
                Ok(metadata) => metadata.len(),
                Err(e) => {
                    cx.update_entity(&view, |_this, cx| {
                        cx.emit(AttachmentButtonEvent::Error(Box::new(
                            AttachmentError::FileReadError(e.to_string()),
                        )));
                    });
                    return;
                },
            };

            let file_type =
                file_path.extension().and_then(|ext| ext.to_str()).map(|s| s.to_string());

            let attachment = AttachmentModel {
                id: Uuid::new_v4().to_string(),
                item_id,
                file_name: file_name.to_string(),
                file_path: file_path.to_string_lossy().to_string(),
                file_type,
                file_size,
            };

            cx.update_entity(&view, |this: &mut AttachmentButtonState, cx| {
                // 先更新本地状态
                this.add_attachment(Arc::new(attachment.clone()), cx);

                if is_temp_id {
                    // 如果是临时 ID，将附件添加到待保存列表
                    tracing::info!(
                        "Item ID is temporary ({}), deferring attachment save",
                        attachment.item_id
                    );
                    this.pending_attachments.push(attachment);
                } else {
                    // 如果是真实 ID，立即保存到数据库
                    tracing::info!(
                        "Item ID is real ({}), saving attachment immediately",
                        attachment.item_id
                    );
                    crate::todo_actions::add_attachment(attachment, cx);
                }
            });
        })
        .detach();
    }

    fn on_remove_attachment(&mut self, attachment_id: &str, cx: &mut Context<Self>) {
        self.remove_attachment(attachment_id, cx);
        delete_attachment(attachment_id.to_string(), cx);
    }
}

impl Render for AttachmentButtonState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let view = cx.entity();
        let search_input = self.search.search_input.clone();
        let filtered_attachments = self.get_filtered_attachments();

        Popover::new("attachment-popover")
            .p_0()
            .text_sm()
            .open(self.search.popover_open)
            .on_open_change(cx.listener(move |this, open, _, cx| {
                manage_popover_state(
                    &mut this.search.popover_open,
                    &mut this.search.search_query,
                    *open,
                );
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
                    .gap_2()
                    .p_2()
                    .w_96()
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(Input::new(&search_input).flex_1())
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
                    .child(v_flex().gap_1().children(filtered_attachments.iter().enumerate().map(
                        |(idx, attachment)| {
                            let attachment_id = attachment.id.clone();
                            let view = view.clone();
                            let display_text = attachment.file_name.clone();

                            create_list_item_element(
                                idx,
                                display_text,
                                attachment_id,
                                view,
                                move |item_id: String,
                                      view: Entity<AttachmentButtonState>,
                                      cx: &mut App| {
                                    cx.update_entity(&view, |this, cx| {
                                        this.on_remove_attachment(&item_id, cx);
                                    });
                                },
                            )
                        },
                    ))),
            )
    }
}

create_button_wrapper!(AttachmentButton, AttachmentButtonState, "item-attachment");
