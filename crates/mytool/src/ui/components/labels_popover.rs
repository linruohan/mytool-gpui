use std::sync::Arc;

use gpui::{
    App, AppContext, BorrowAppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, Styled, Subscription, Window, actions,
    px,
};
use gpui_component::{
    Sizable,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState},
    list::{List, ListState},
    popover::Popover,
    separator::Separator,
    v_flex,
};
use gpui_kit::assets::IconName;
use todos::entity::LabelModel;
use tracing::info;

use crate::{
    LabelCheckListDelegate, SelectedCheckLabel, UnSelectedCheckLabel, todo_state::TodoStore,
};

actions!(labels_popover, [CreateNewLabel]);

pub enum LabelsPopoverEvent {
    LabelsChanged(String),
}

pub struct LabelsPopoverList {
    focus_handle: FocusHandle,
    pub label_list: Entity<ListState<LabelCheckListDelegate>>,
    pub selected_labels: Vec<Arc<LabelModel>>,
    pub(crate) list_popover_open: bool,
    _subscriptions: Vec<Subscription>,
    new_label_input: Entity<InputState>, // 新增标签输入框
}
impl EventEmitter<LabelsPopoverEvent> for LabelsPopoverList {}
impl LabelsPopoverList {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let label_list = cx.new(|cx| {
            ListState::new(LabelCheckListDelegate::new(), window, cx)
                .searchable(true)
                .selectable(true)
        });

        // 创建新标签输入框
        let new_label_input = cx.new(|cx| InputState::new(window, cx).placeholder("新标签名称"));

        cx.focus_self(window);
        let label_list_clone = label_list.clone();

        // 初始化全局标签
        let initial_labels = cx.global::<TodoStore>().labels.clone();
        cx.update_entity(&label_list_clone, |list, cx| {
            list.delegate_mut().update_labels(initial_labels);
            // 初始化时设置空的 checked 状态，确保所有标签默认未选中
            list.delegate_mut().set_item_checked_labels(Vec::new(), cx);
            cx.notify();
        });

        let _subscriptions = vec![
            cx.observe_global::<TodoStore>(move |_this, cx| {
                let labels = {
                    let store = cx.global::<TodoStore>();
                    if !store.peek_change_mask().affects_label_list() {
                        return;
                    }
                    store.labels.clone()
                };
                cx.update_entity(&label_list_clone, |list, cx| {
                    list.delegate_mut().update_labels(labels);
                    cx.notify();
                });
                cx.notify();
            }),
            cx.subscribe_in(&new_label_input, window, Self::on_new_label_input_event),
        ];
        Self {
            list_popover_open: false,
            label_list,
            focus_handle: cx.focus_handle(),
            selected_labels: Vec::new(),
            _subscriptions,
            new_label_input,
        }
    }

    pub fn set_item_checked_label_id(
        &mut self,
        label_ids: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_item_checked_label_id_async(label_ids, cx);
    }

    pub fn set_item_checked_label_id_async(&mut self, label_ids: String, cx: &mut Context<Self>) {
        let selected = {
            let store = cx.global::<TodoStore>();
            label_ids
                .split(';')
                .filter_map(|label_id| {
                    let trimmed_id = label_id.trim();
                    if trimmed_id.is_empty() {
                        return None;
                    }
                    store.get_label(trimmed_id).or_else(|| {
                        store.labels.iter().find(|label| label.name == trimmed_id).cloned()
                    })
                })
                .collect()
        };
        self.selected_labels = selected;
        self.label_list.update(cx, |list, cx| {
            list.delegate_mut().set_item_checked_labels(self.selected_labels.clone(), cx);
        });
    }

    fn update_label_selection(&mut self, select: bool, cx: &mut Context<Self>) {
        info!("update_label_selection called: select={}", select);
        let picker = self.label_list.read(cx);
        if let Some(label) = picker.delegate().selected_label() {
            info!("update_label_selection: selected label={}", label.name);
            let contains = self.selected_labels.iter().any(|l| l.id == label.id);
            info!("update_label_selection: contains={}, select={}", contains, select);
            if (select && !contains) || (!select && contains) {
                if select {
                    self.selected_labels.push(label.clone());
                } else {
                    self.selected_labels.retain(|l| l.id != label.id);
                }
                // 同步更新 LabelCheckListDelegate 的 checked_list
                self.label_list.update(cx, |list, cx| {
                    list.delegate_mut().set_item_checked_labels(self.selected_labels.clone(), cx);
                });
                // 发送标签ID字符串
                self.emit_labels_changed(cx);
            } else {
                info!("update_label_selection: condition not met, skipping emit_labels_changed");
            }
            // 移除cx.notify()调用，避免每次点击标签都重新渲染组件导致popover关闭
        } else {
            info!("update_label_selection: no selected label found");
        }
    }

    fn selected_label(
        &mut self,
        _: &SelectedCheckLabel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        info!("selected_label action received");
        self.update_label_selection(true, cx);
    }

    fn unselected_label(
        &mut self,
        _: &UnSelectedCheckLabel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        info!("unselected_label action received");
        self.update_label_selection(false, cx);
    }

    // 处理新标签输入框事件
    fn on_new_label_input_event(
        &mut self,
        _state: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let InputEvent::PressEnter { .. } = event {
            let label_name = self.new_label_input.read(cx).value().to_string();
            if !label_name.trim().is_empty() {
                self.create_new_label(label_name, window, cx);
            }
        }
    }

    // 创建新标签
    fn create_new_label(
        &mut self,
        label_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 生成随机颜色
        // 柔和色板，避免荧光黄/青导致字看不清
        let colors = [
            "#ef5350", "#ec407a", "#ab47bc", "#5c6bc0", "#42a5f5", "#26a69a", "#66bb6a", "#9ccc65",
            "#ffca28", "#ffa726", "#ff7043", "#8d6e63",
        ];
        let color_seed =
            label_name.bytes().fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize));
        let color = colors[color_seed % colors.len()].to_string();

        // 创建新标签模型
        let new_label = Arc::new(LabelModel {
            id: uuid::Uuid::new_v4().to_string(),
            name: label_name.trim().to_string(),
            color,
            item_order: 0,
            is_deleted: false,
            is_favorite: false,
            backend_type: None,
            source_id: None,
        });

        cx.update_global::<TodoStore, _>(|store, _| {
            store.add_label(new_label.clone());
        });

        self.new_label_input.update(cx, |input, cx| {
            input.set_value("".to_string(), window, cx);
        });

        if !self.selected_labels.iter().any(|l| l.id == new_label.id) {
            self.selected_labels.push(new_label.clone());
            self.label_list.update(cx, |list, cx| {
                list.delegate_mut().set_item_checked_labels(self.selected_labels.clone(), cx);
            });
        }
        cx.notify();

        let db_state = cx.global::<crate::todo_state::DBState>().clone();
        let label_for_db = new_label.as_ref().clone();
        let label_id = new_label.id.clone();
        cx.spawn(async move |this, cx| {
            match db_state
                .spawn_store_op(move |store| async move { store.insert_label(label_for_db).await })
                .await
            {
                Ok(Ok(_)) => {
                    this.update(cx, |this, cx| {
                        this.emit_labels_changed(cx);
                    })
                    .ok();
                },
                Ok(Err(e)) => {
                    tracing::error!("insert_label failed: {:?}", e);
                    this.update(cx, |this, cx| {
                        this.selected_labels.retain(|l| l.id != label_id);
                        cx.update_global::<TodoStore, _>(|store, _| {
                            store.remove_label(&label_id);
                        });
                        cx.notify();
                    })
                    .ok();
                },
                Err(join_err) => {
                    tracing::error!("insert_label task panicked: {:?}", join_err);
                },
            }
        })
        .detach();
    }

    // 发送标签变更事件
    fn emit_labels_changed(&self, cx: &mut Context<Self>) {
        let label_ids =
            self.selected_labels.iter().map(|label| label.id.clone()).collect::<Vec<_>>().join(";");
        info!(
            "emit_labels_changed: selected_labels count: {}, label_ids: '{}'",
            self.selected_labels.len(),
            label_ids
        );
        cx.emit(LabelsPopoverEvent::LabelsChanged(label_ids));
    }

    // 获取选中的标签ID字符串
    pub fn get_selected_label_ids(&self) -> String {
        self.selected_labels.iter().map(|label| label.id.clone()).collect::<Vec<_>>().join(";")
    }
}

impl Focusable for LabelsPopoverList {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
const CONTEXT: &str = "label-popover-list";
impl Render for LabelsPopoverList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_count = self.selected_labels.len();

        v_flex()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_label))
            .on_action(cx.listener(Self::unselected_label))
            .child(
                Popover::new("popover-list")
                    .p_0()
                    .text_sm()
                    .open(self.list_popover_open)
                    .on_open_change(cx.listener(move |this, open, window, cx| {
                        this.list_popover_open = *open;
                        if *open {
                            this.label_list.update(cx, |list, cx| {
                                list.focus(window, cx);
                            });
                        }
                        cx.notify();
                    }))
                    .trigger({
                        let mut button = Button::new("item-labels-button")
                            .small()
                            .ghost()
                            .compact()
                            .tooltip(if selected_count > 0 {
                                format!("已选 {} 个标签", selected_count)
                            } else {
                                "设置标签".to_string()
                            })
                            .icon(IconName::TagOutlineSymbolic);
                        if selected_count > 0 {
                            button = button.label(format!("{}", selected_count));
                        }
                        button
                    })
                    .child(
                        v_flex()
                            .gap_1()
                            .p_1p5()
                            .w_full()
                            .child(
                                List::new(&self.label_list)
                                    .search_placeholder("搜索标签")
                                    .scrollbar_visible(false)
                                    .max_h(px(180.)),
                            )
                            .child(Separator::horizontal())
                            .child(
                                Input::new(&self.new_label_input).small().suffix(
                                    Button::new("create-label-button")
                                        .small()
                                        .ghost()
                                        .compact()
                                        .icon(IconName::Plus)
                                        .tooltip("创建标签")
                                        .on_click(cx.listener(|this, _event, window, cx| {
                                            let label_name =
                                                this.new_label_input.read(cx).value().to_string();
                                            if !label_name.trim().is_empty() {
                                                this.create_new_label(label_name, window, cx);
                                            }
                                        })),
                                ),
                            ),
                    )
                    .w(px(220.)),
            )
    }
}
