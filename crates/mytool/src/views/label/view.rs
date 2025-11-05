use super::LabelEvent;
use crate::{DBState, LabelListDelegate, load_labels};
use gpui::{
    App, AppContext, Context, Entity, EventEmitter, IntoElement, ParentElement, Render, Styled,
    Subscription, WeakEntity, Window, px,
};
use gpui_component::{
    ActiveTheme,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    input::{Input, InputState},
    list::{List, ListEvent, ListState},
    {ContextModal, IndexPath, v_flex},
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
            cx.new(|cx| ListState::new(LabelListDelegate::new(), window, cx).searchable(true));

        let _subscriptions = vec![cx.subscribe_in(
            &label_list,
            window,
            |this, _, ev: &ListEvent, window, cx| {
                if let ListEvent::Confirm(ix) = ev
                    && let Some(conn) = this.get_selected_label(*ix, cx)
                {
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
            println!("len labels: {}", labels.len());
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
    pub fn add_label_model(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Project Name"));
        let _input2 = cx.new(|cx| -> InputState {
            InputState::new(window, cx).placeholder("For test focus back on modal close.")
        });
        let now = chrono::Local::now().naive_local().date();
        let label_due = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
            picker.set_date(now, window, cx);
            picker
        });
        let _ = cx.subscribe(&label_due, |this, _, ev, _| match ev {
            DatePickerEvent::Change(date) => {
                this.label_due = date.format("%Y-%m-%d").map(|s| s.to_string());
            }
        });
        let view = cx.entity().clone();

        window.open_modal(cx, move |modal, _, _| {
            modal
                .title("Add Project")
                .overlay(false)
                .keyboard(true)
                .show_close(true)
                .overlay_closable(true)
                .child(
                    v_flex()
                        .gap_3()
                        .child(Input::new(&input1))
                        .child(DatePicker::new(&label_due).placeholder("DueDate of Project")),
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
                                    window.close_modal(cx);
                                    view.update(cx, |_view, cx| {
                                        let label = LabelModel {
                                            id: "".to_string(),
                                            name: input1.read(cx).value().to_string(),
                                            // due_date: view.label_due.clone(),
                                            // ..Default::default()
                                            color: "".to_string(),
                                            item_order: 0,
                                            is_deleted: false,
                                            is_favorite: false,
                                            backend_type: None,
                                            source_id: None,
                                        };
                                        cx.emit(LabelEvent::Added(label.into()));
                                        cx.notify();
                                    });
                                }
                            }),
                            Button::new("cancel")
                                .label("Cancel")
                                .on_click(move |_, window, cx| {
                                    window.close_modal(cx);
                                }),
                        ]
                    }
                })
        });
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
            .p(px(8.))
            .flex_1()
            .w_full()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
    }
}
