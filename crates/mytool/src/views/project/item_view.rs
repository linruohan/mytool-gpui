use std::rc::Rc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Subscription, WeakEntity, Window, div,
};
use gpui_component::{
    ActiveTheme, IconName, IndexPath, WindowExt,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    h_flex,
    input::{Input, InputState},
    list::{List, ListEvent, ListState},
    menu::{DropdownMenu, PopupMenuItem},
    v_flex,
};
use todos::entity::{ItemModel, ProjectModel};

use crate::{DBState, ItemListDelegate, get_project_items};
pub enum ProjectItemEvent {
    Loaded,
    Added(Rc<ItemModel>),
    Modified(Rc<ItemModel>),
    Deleted(Rc<ItemModel>),
}
impl EventEmitter<ProjectItemEvent> for ProjectItemsPanel {}
pub struct ProjectItemsPanel {
    pub item_list: Entity<ListState<ItemListDelegate>>,
    project: Rc<ProjectModel>,
    is_loading: bool,
    item_due: Option<String>,
    pub active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl ProjectItemsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item_list =
            cx.new(|cx| ListState::new(ItemListDelegate::new(), window, cx).searchable(true));

        let _subscriptions = vec![cx.subscribe(&item_list, |_, _, ev: &ListEvent, _| match ev {
            ListEvent::Select(ix) => {
                println!("ProjectItemsPanel List Selected: {:?}", ix);
            },
            ListEvent::Confirm(ix) => {
                println!("ProjectItemsPanel List Confirmed: {:?}", ix);
            },
            ListEvent::Cancel => {
                println!("ProjectItemsPanel List Cancelled");
            },
        })];
        // let item_list_clone = item_list.clone();
        // let db = cx.global::<DBState>().conn.clone();

        // cx.spawn(async move |_view, cx| {
        //     let db = db.lock().await;
        //     let items = get_items_by_project_id(&project_clone.id, db.clone()).await;
        //     let rc_items: Vec<Rc<ItemModel>> =
        //         items.iter().map(|pro| Rc::new(pro.clone())).collect();
        //     println!("len items: {}", items.len());
        //     let _ = cx
        //         .update_entity(&item_list_clone, |list, cx| {
        //             list.delegate_mut().update_items(rc_items);
        //             cx.notify();
        //         })
        //         .ok();
        // })
        // .detach();
        Self {
            item_due: None,
            is_loading: true,
            item_list,
            active_index: Some(0),
            _subscriptions,
            // project: project.clone(),
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

    pub fn set_project(&mut self, project: Rc<ProjectModel>, cx: &mut Context<Self>) {
        self.project = project;
        self.update_items(cx);
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
                println!("handle_item_event:");
                self.add_item(cx, item.clone())
            },
            ProjectItemEvent::Modified(item) => self.mod_item(cx, item.clone()),
            ProjectItemEvent::Deleted(item) => self.del_item(cx, item.clone()),
            _ => {},
        }
    }

    pub fn show_model(
        &mut self,
        _model: Rc<ItemModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Project Name"));
        let _input2 = cx.new(|cx| -> InputState {
            InputState::new(window, cx).placeholder("For test focus back on modal close.")
        });
        let now = chrono::Local::now().naive_local().date();
        let item_due = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
            picker.set_date(now, window, cx);
            picker
        });
        let _ = cx.subscribe(&item_due, |this, _, ev, _| match ev {
            DatePickerEvent::Change(date) => {
                this.item_due = date.format("%Y-%m-%d").map(|s| s.to_string());
            },
        });
        let view = cx.entity().clone();

        window.open_dialog(cx, move |modal, _, _| {
            modal
                .title("Add Item")
                .overlay(false)
                .keyboard(true)
                .overlay_closable(true)
                .child(
                    v_flex()
                        .gap_3()
                        .child(Input::new(&input1))
                        .child(DatePicker::new(&item_due).placeholder("DueDate of Project")),
                )
                .footer({
                    let view = view.clone();
                    let input1 = input1.clone();
                    move |_, _, _, _cx| {
                        vec![
                            Button::new("add").primary().label("Add").on_click({
                                let view = view.clone();
                                let input1 = input1.clone();
                                move |_, window, cx| {
                                    window.close_dialog(cx);
                                    view.update(cx, |_view, cx| {
                                        let item = ItemModel {
                                            content: input1.read(cx).value().to_string(),
                                            ..Default::default()
                                        };
                                        cx.emit(ProjectItemEvent::Added(item.into()));
                                        cx.notify();
                                    });
                                }
                            }),
                            Button::new("cancel").label("Cancel").on_click(move |_, window, cx| {
                                window.close_dialog(cx);
                            }),
                        ]
                    }
                })
        });
    }

    // 更新items
    pub fn update_items(&mut self, cx: &mut Context<Self>) {
        if !self.is_loading {
            return;
        }
        let db = cx.global::<DBState>().conn.clone();
        let project = self.project.clone();
        cx.spawn(async move |this, cx| {
            let db = db.lock().await;
            let items = get_project_items(project, db.clone()).await;
            let rc_items: Vec<Rc<ItemModel>> =
                items.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("project's items: {:?}", rc_items.len());
            this.update(cx, |this, cx| {
                this.item_list.update(cx, |list, cx| {
                    list.delegate_mut().update_items(rc_items);
                    cx.notify();
                });

                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn add_item(&mut self, cx: &mut Context<Self>, item: Rc<ItemModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ProjectItemsPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::add_item(item.clone(), db.clone()).await;
            println!("add_item {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.update_items(cx);
    }

    pub fn mod_item(&mut self, cx: &mut Context<Self>, item: Rc<ItemModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ProjectItemsPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::mod_item(item.clone(), db.clone()).await;
            println!("mod_item {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.update_items(cx);
    }

    pub fn del_item(&mut self, cx: &mut Context<Self>, item: Rc<ItemModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ProjectItemsPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::del_item(item.clone(), db.clone()).await;
            println!("mod_item {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.update_items(cx);
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
                                                    this.show_model(model, window, cx);
                                                } else {
                                                    this.show_model(
                                                        Rc::new(ItemModel::default()),
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
                                                            this.del_item(cx, item.clone());
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
