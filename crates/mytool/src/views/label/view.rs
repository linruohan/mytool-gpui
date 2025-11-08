use super::LabelEvent;
use crate::{DBState, LabelListDelegate, load_labels};
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, Render, Styled,
    Subscription, WeakEntity, Window, px,
};
use gpui_component::{
    ActiveTheme, IndexPath, WindowExt,
    button::{Button, ButtonVariants},
    input::{Input, InputState},
    list::{List, ListEvent, ListState},
    v_flex,
};
use std::rc::Rc;
use todos::entity::LabelModel;

impl EventEmitter<LabelEvent> for LabelsPanel {}
pub struct LabelsPanel {
    input_esc: Entity<InputState>,
    pub label_list: Entity<ListState<LabelListDelegate>>,
    label_due: Option<String>,
    is_loading: bool,
    pub active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl LabelsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_esc = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter DB URL")
                .clean_on_escape()
        });

        let label_list =
            cx.new(|cx| ListState::new(LabelListDelegate::new(), window, cx).selectable(true));

        let _subscriptions = vec![cx.subscribe_in(
            &label_list,
            window,
            |this, _, ev: &ListEvent, window, cx| {
                if let ListEvent::Confirm(ix) = ev
                    && let Some(conn) = this.get_selected_label(*ix, cx)
                {
                    this.update_active_index(Some(ix.row));
                    this.input_esc.update(cx, |is, cx| {
                        is.set_value(conn.clone().name.clone(), window, cx);
                        cx.notify();
                    })
                }
            },
        )];

        let label_list_clone = label_list.clone();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |_view, cx| {
            let db = db.lock().await;
            let labels = load_labels(db.clone()).await;
            let rc_labels: Vec<Rc<LabelModel>> =
                labels.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("label_panel: len labels: {}", labels.len());
            let _ = cx
                .update_entity(&label_list_clone, |list, cx| {
                    list.delegate_mut().update_labels(rc_labels);
                    cx.notify();
                })
                .ok();
        })
        .detach();
        Self {
            input_esc,
            is_loading: false,
            label_list,
            label_due: None,
            active_index: Some(0),
            _subscriptions,
        }
    }
    fn get_selected_label(&self, ix: IndexPath, cx: &App) -> Option<Rc<LabelModel>> {
        self.label_list
            .read(cx)
            .delegate()
            .matched_labels
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }
    pub fn update_active_index(&mut self, value: Option<usize>) {
        self.active_index = value;
    }
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
    pub fn handle_label_event(&mut self, event: &LabelEvent, cx: &mut Context<Self>) {
        match event {
            LabelEvent::Loaded => {
                println!("Loaded");
                self.get_labels(cx);
            }
            LabelEvent::Added(label) => {
                println!("handle_label_event:");
                self.add_label(cx, label.clone())
            }
            LabelEvent::Modified(label) => self.mod_label(cx, label.clone()),
            LabelEvent::Deleted(label) => self.del_label(cx, label.clone()),
        }
    }
    pub fn show_label_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        is_edit: bool,
    ) {
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Label Name"));

        // 如果是编辑模式，预填充当前标签名
        if is_edit {
            if let Some(active_index) = self.active_index {
                println!("show_label_dialog: active index: {}", active_index);
                let label_some = self.get_selected_label(IndexPath::new(active_index), &cx);
                if let Some(label) = label_some {
                    name_input.update(cx, |is, cx| {
                        is.set_value(label.name.clone(), window, cx);
                        cx.notify();
                    })
                }
            }
        }

        let view = cx.entity().clone();
        let dialog_title = if is_edit { "Edit Label" } else { "Add Label" };
        let button_label = if is_edit { "Save" } else { "Add" };

        window.open_dialog(cx, move |modal, _, _| {
            modal
                .title(dialog_title)
                .overlay(false)
                .keyboard(true)
                .overlay_closable(true)
                .child(v_flex().gap_3().child(Input::new(&name_input)))
                .footer({
                    let view = view.clone();
                    let name_input_clone = name_input.clone();
                    move |_, _, _, _cx| {
                        vec![
                            Button::new("save").primary().label(button_label).on_click({
                                let view = view.clone();
                                let name_input_clone1 = name_input_clone.clone();
                                move |_, window, cx| {
                                    window.close_dialog(cx);
                                    view.update(cx, |_view, cx| {
                                        let label = LabelModel {
                                            name: name_input_clone1.read(cx).value().to_string(),
                                            ..Default::default()
                                        };
                                        // 根据模式发射不同事件
                                        if is_edit {
                                            cx.emit(LabelEvent::Modified(label.into()));
                                        } else {
                                            cx.emit(LabelEvent::Added(label.into()));
                                        }
                                        cx.notify();
                                    });
                                }
                            }),
                            Button::new("cancel")
                                .label("Cancel")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                }),
                        ]
                    }
                })
        });
    }
    pub fn show_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.active_index {
            let label_some = self.get_selected_label(IndexPath::new(active_index), &cx);
            if let Some(label) = label_some {
                let view = cx.entity().clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .confirm()
                        .overlay(true)
                        .overlay_closable(true)
                        .child("Are you sure to delete the label?")
                        .on_ok({
                            let view = view.clone();
                            let label = label.clone();
                            move |_, window, cx| {
                                let view = view.clone();
                                let label = label.clone();
                                view.update(cx, |_view, cx| {
                                    cx.emit(LabelEvent::Deleted(label));
                                    cx.notify();
                                });
                                window.push_notification("You have delete ok.", cx);
                                true
                            }
                        })
                        .on_cancel(|_, window, cx| {
                            window.push_notification("You have canceled delete.", cx);
                            true
                        })
                });
            };
        }
    }
    // 更新labels
    pub fn get_labels(&mut self, cx: &mut Context<Self>) {
        if !self.is_loading {
            return;
        }
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this, cx| {
            let db = db.lock().await;
            let labels = load_labels(db.clone()).await;
            let rc_labels: Vec<Rc<LabelModel>> =
                labels.iter().map(|pro| Rc::new(pro.clone())).collect();

            this.update(cx, |this, cx| {
                this.label_list.update(cx, |list, cx| {
                    list.delegate_mut().update_labels(rc_labels);
                    cx.notify();
                });

                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn add_label(&mut self, cx: &mut Context<Self>, label: Rc<LabelModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<LabelsPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::add_label(label.clone(), db.clone()).await;
            println!("add_label {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.get_labels(cx);
    }
    pub fn mod_label(&mut self, cx: &mut Context<Self>, label: Rc<LabelModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<LabelsPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::mod_label(label.clone(), db.clone()).await;
            println!("mod_label {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.get_labels(cx);
    }
    pub fn del_label(&mut self, cx: &mut Context<Self>, label: Rc<LabelModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<LabelsPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::del_label(label.clone(), db.clone()).await;
            println!("mod_label {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.get_labels(cx);
    }
}

impl Render for LabelsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        List::new(&self.label_list)
            .p(px(2.))
            .flex_1()
            .w_full()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
    }
}
