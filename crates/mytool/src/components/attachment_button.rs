use std::rc::Rc;

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
    components::{PopoverListMixin, PopoverSearchMixin},
    create_button_wrapper,
    todo_actions::delete_attachment,
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

pub enum AttachmentButtonEvent {
    Added(Rc<AttachmentModel>),
    Removed(String),
    Error(AttachmentError),
}

pub struct AttachmentButtonState {
    focus_handle: FocusHandle,
    pub item_id: String,
    search: PopoverSearchMixin,
    items: PopoverListMixin<Rc<AttachmentModel>>,
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

        let filter_fn = |attachment: &Rc<AttachmentModel>, query: &str| {
            attachment.file_name.to_lowercase().contains(&query.to_lowercase())
        };

        Self {
            focus_handle: cx.focus_handle(),
            item_id,
            search: PopoverSearchMixin::new(search_input),
            items: PopoverListMixin::new(filter_fn),
        }
    }

    pub fn set_attachments(
        &mut self,
        attachments: Vec<Rc<AttachmentModel>>,
        cx: &mut Context<Self>,
    ) {
        self.items.set_items(attachments);
        cx.notify();
    }

    pub fn add_attachment(&mut self, attachment: Rc<AttachmentModel>, cx: &mut Context<Self>) {
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
            let query = self.search.search_input.read(cx).value().to_string();
            self.search.update_search_query(query);
            cx.notify();
        }
    }

    fn get_filtered_attachments(&self) -> Vec<Rc<AttachmentModel>> {
        self.items.get_filtered(&self.search.search_query)
    }

    fn on_add_attachment(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Err(e) = self.try_add_attachment(cx) {
            cx.emit(AttachmentButtonEvent::Error(e));
        }
    }

    fn try_add_attachment(&mut self, cx: &mut Context<Self>) -> AttachmentResult<()> {
        let file_path = rfd::FileDialog::new().pick_file();
        let file_path = file_path
            .ok_or_else(|| AttachmentError::FileNotFound("No file selected".to_string()))?;

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(AttachmentError::InvalidFileName)?;

        let file_size = std::fs::metadata(&file_path)
            .map(|m| m.len())
            .map_err(|e| AttachmentError::FileReadError(e.to_string()))?;

        let file_type = file_path.extension().and_then(|ext| ext.to_str()).map(|s| s.to_string());

        let attachment = AttachmentModel {
            id: Uuid::new_v4().to_string(),
            item_id: self.item_id.clone(),
            file_name: file_name.to_string(),
            file_path: file_path.to_string_lossy().to_string(),
            file_type,
            file_size,
        };

        self.add_attachment(Rc::new(attachment.clone()), cx);
        crate::todo_actions::add_attachment(attachment, cx);
        Ok(())
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
                this.search.popover_open = *open;
                if !*open {
                    this.search.clear_search();
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
                    .child(
                        h_flex()
                            .gap_2()
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
                    .child(v_flex().gap_2().children(filtered_attachments.iter().enumerate().map(
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
                                    gpui_component::label::Label::new(attachment.file_name.clone())
                                        .text_sm(),
                                )
                                .child(
                                    Button::new(format!("remove-attachment-dialog-{}", idx))
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
                        },
                    ))),
            )
    }
}

create_button_wrapper!(AttachmentButton, AttachmentButtonState, "item-attachment");
