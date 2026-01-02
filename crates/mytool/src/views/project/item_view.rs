use std::rc::Rc;

use gpui::{
    div, App, AppContext, Context, Entity, EventEmitter, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Subscription, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants}, h_flex, list::{List, ListEvent, ListState}, menu::{DropdownMenu, PopupMenuItem},
    v_flex,
    ActiveTheme,
    IconName,
    IndexPath,
    WindowExt,
};
use todos::entity::{ItemModel, ProjectModel};

use crate::{
    todo_actions::{add_project_item, delete_project_item, update_project_item}, todo_state::ProjectItemState, ItemEvent, ItemInfo, ItemInfoEvent,
    ItemInfoState,
    ItemListDelegate,
};

pub enum ProjectItemEvent {
    Loaded,
    Added(Rc<ItemModel>),
    Modified(Rc<ItemModel>),
    Deleted(Rc<ItemModel>),
}
impl EventEmitter<ProjectItemEvent> for ProjectItemsPanel {}
impl EventEmitter<ItemInfoEvent> for ProjectItemsPanel {}
impl EventEmitter<ItemEvent> for ProjectItemsPanel {}
pub struct ProjectItemsPanel {
    pub item_list: Entity<ListState<ItemListDelegate>>,
    project: Rc<ProjectModel>,
    pub active_index: Option<usize>,
    item_info: Entity<ItemInfoState>,
    _subscriptions: Vec<Subscription>,
}

impl ProjectItemsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = Rc::new(ItemModel::default());
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
        let item_list =
            cx.new(|cx| ListState::new(ItemListDelegate::new(), window, cx).searchable(true));
        let item_list_clone = item_list.clone();
        let _subscriptions = vec![
            cx.observe_global::<ProjectItemState>(move |_this, cx| {
                let items = cx.global::<ProjectItemState>().items.clone();
                cx.update_entity(&item_list_clone, |list, cx| {
                    list.delegate_mut().update_items(items);
                    cx.notify();
                });
                cx.notify();
            }),
            cx.subscribe(&item_info, |_this, _, event: &ItemInfoEvent, cx| match event {
                ItemInfoEvent::Updated(item) => {
                    print!("iteminfo updated after:{:?}", item);
                    cx.emit(ItemEvent::Modified(item.clone()));
                    cx.notify();
                },
                ItemInfoEvent::Added(item) => {
                    cx.emit(ItemEvent::Added(item.clone()));
                    cx.notify();
                },
                ItemInfoEvent::Deleted(item) => {
                    cx.emit(ItemEvent::Deleted(item.clone()));
                },
                ItemInfoEvent::Finished(item) => {
                    cx.emit(ItemEvent::Finished(item.clone()));
                },
                ItemInfoEvent::UnFinished(item) => {
                    cx.emit(ItemEvent::Modified(item.clone()));
                },
            }),
            cx.subscribe_in(&item_list, window, |this, _, ev: &ListEvent, _window, cx| {
                if let ListEvent::Confirm(ix) = ev
                    && let Some(_item) = this.get_selected_item(*ix, cx)
                {
                    this.update_active_index(Some(ix.row));
                }
            }),
        ];

        Self {
            item_list,
            active_index: Some(0),
            item_info,
            _subscriptions,
            project: Rc::new(ProjectModel::default()),
        }
    }

    pub(crate) fn get_selected_item(&self, ix: IndexPath, cx: &App) -> Option<Rc<ItemModel>> {
        self.item_list
            .read(cx)
            .delegate()
            .matched_items
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }

    pub fn set_project(&mut self, project: Rc<ProjectModel>, _cx: &mut Context<Self>) {
        self.project = project;
        // self.update_items(cx);
    }

    pub fn update_active_index(&mut self, value: Option<usize>) {
        self.active_index = value;
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn handle_project_item_event(&mut self, event: &ProjectItemEvent, cx: &mut Context<Self>) {
        match event {
            ProjectItemEvent::Added(item) => {
                add_project_item(self.project.clone(), item.clone(), cx);
            },
            ProjectItemEvent::Modified(item) => {
                update_project_item(self.project.clone(), item.clone(), cx)
            },
            ProjectItemEvent::Deleted(item) => {
                delete_project_item(self.project.clone(), item.clone(), cx)
            },
            _ => {},
        }
    }

    fn initialize_item_model(&self, _is_edit: bool, _: &mut Window, cx: &mut App) -> ItemModel {
        self.active_index
            .and_then(|index| {
                println!("show_label_dialog: active index: {}", index);
                self.get_selected_item(IndexPath::new(index), cx)
            })
            .map(|label| {
                let item_ref = label.as_ref();
                ItemModel { ..item_ref.clone() }
            })
            .unwrap_or_default()
    }

    pub fn show_model(
        &mut self,
        _model: Rc<ItemModel>,
        is_edit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let item_info = self.item_info.clone();
        let ori_item = self.initialize_item_model(is_edit, window, cx);
        if is_edit {
            item_info.update(cx, |state, cx| {
                state.set_item(Rc::new(ori_item.clone()), window, cx);
                cx.notify();
            });
        } else {
        }
        let view = cx.entity().clone();
        let dialog_title = if is_edit { "Edit Item" } else { "New Item" };
        let button_text = if is_edit { "Save" } else { "Add" };

        window.open_dialog(cx, move |modal, _, _| {
            let item_info_clone = item_info.clone();
            let view_clone = view.clone();

            modal
                .title(dialog_title)
                .overlay(true)
                .keyboard(true)
                .overlay_closable(true)
                .child(ItemInfo::new(&item_info))
                .footer(move |_, _, _, _| {
                    vec![
                        Button::new("save").primary().label(button_text).on_click({
                            let view = view_clone.clone();
                            let item_info = item_info_clone.clone();
                            move |_, window, cx| {
                                window.close_dialog(cx);
                                item_info.update(cx, |item_info, cx| {
                                    cx.emit(ItemInfoEvent::Updated(item_info.item.clone()));
                                    cx.notify();
                                });
                                view.update(cx, |_view, cx| {
                                    let item = item_info.read(cx).item.clone();
                                    print!("iteminfo dialog: {:?}", item.clone());
                                    let event = if is_edit {
                                        ProjectItemEvent::Modified(item.clone())
                                    } else {
                                        ProjectItemEvent::Added(item.clone())
                                    };
                                    cx.emit(event);
                                    cx.notify();
                                });
                            }
                        }),
                        Button::new("cancel").label("Cancel").on_click(move |_, window, cx| {
                            window.close_dialog(cx);
                        }),
                    ]
                })
        });
    }
}

impl Render for ProjectItemsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _items: Vec<_> = self.item_list.read(cx).delegate()._items.clone();
        let view = cx.entity();
        v_flex()
            .size_full()
            .gap_4()
            .child(
                h_flex()
                    .id("header")
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .justify_between()
                    .items_start()
                    .child(v_flex().child(div().text_xl().child(self.project.name.clone())))
                    .child(
                        div().items_end().text_color(cx.theme().muted_foreground).child(
                            Button::new("item-popup-menu")
                                .icon(IconName::EllipsisVertical)
                                .dropdown_menu({
                                    let view = view.clone();
                                    move |this, window, _cx| {
                                        this.link(
                                            "About",
                                            "https://github.com/longbridge/gpui-component",
                                        )
                                        .separator()
                                        .item(PopupMenuItem::new("Edit item").on_click(
                                            window.listener_for(&view, |this, _, window, cx| {
                                                if let Some(model) =
                                                    this.active_index.map(IndexPath::new).and_then(
                                                        |index| this.get_selected_item(index, cx),
                                                    )
                                                {
                                                    this.show_model(model, false, window, cx);
                                                } else {
                                                    this.show_model(
                                                        Rc::new(ItemModel::default()),
                                                        true,
                                                        window,
                                                        cx,
                                                    );
                                                }
                                                cx.notify();
                                            }),
                                        ))
                                        .separator()
                                        .item(
                                            PopupMenuItem::new("Delete item").on_click(
                                                window.listener_for(
                                                    &view,
                                                    |this, _, _window, cx| {
                                                        let index = this.active_index.unwrap();
                                                        let item_some = this.get_selected_item(
                                                            IndexPath::new(index),
                                                            cx,
                                                        );
                                                        if let Some(item) = item_some {
                                                            cx.emit(ProjectItemEvent::Deleted(
                                                                item,
                                                            ));
                                                        }
                                                        cx.notify();
                                                    },
                                                ),
                                            ),
                                        )
                                    }
                                }),
                        ),
                    ),
            )
            .child(List::new(&self.item_list))
    }
}
