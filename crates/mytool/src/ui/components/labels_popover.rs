use std::sync::Arc;

use gpui::{
    App, AppContext, BorrowAppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, Styled, Subscription, Window, actions,
    prelude::FluentBuilder, px,
};
use gpui_component::{
    IconName, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    list::{List, ListState},
    popover::Popover,
    v_flex,
};
use todos::entity::LabelModel;

use crate::{
    LabelCheckListDelegate, SelectedCheckLabel, UnSelectedCheckLabel, todo_state::TodoStore,
};

actions!(labels_popover, [CreateNewLabel]);

pub enum LabelsPopoverEvent {
    Selected(Arc<LabelModel>),
    DeSelected(Arc<LabelModel>),
    LabelsChanged(String), // 新增事件，当标签选择改变时发送标签ID字符串
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
        let parent = cx.entity();
        let label_list = cx.new(|cx| {
            ListState::new(LabelCheckListDelegate::new(parent), window, cx)
                .searchable(true)
                .selectable(true)
        });

        // 创建新标签输入框
        let new_label_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("New label name"));

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
                let labels = cx.global::<TodoStore>().labels.clone();
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
        let all_labels = self.label_list.read(cx).delegate()._labels.clone();
        self.selected_labels = label_ids
            .split(';')
            .filter_map(|label_id| {
                let trimmed_id = label_id.trim();
                if trimmed_id.is_empty() {
                    return None;
                }
                all_labels.iter().find(|label| label.id == trimmed_id).map(Arc::clone)
            })
            .collect();
        self.label_list.update(cx, |list, cx| {
            list.delegate_mut().set_item_checked_labels(self.selected_labels.clone(), cx);
        });
    }

    fn update_label_selection(&mut self, select: bool, cx: &mut Context<Self>) {
        let picker = self.label_list.read(cx);
        if let Some(label) = picker.delegate().selected_label() {
            let contains = self.selected_labels.iter().any(|l| l.id == label.id);
            if (select && !contains) || (!select && contains) {
                if select {
                    self.selected_labels.push(label.clone());
                    cx.emit(LabelsPopoverEvent::Selected(label.clone()));
                } else {
                    self.selected_labels.retain(|l| l.id != label.id);
                    cx.emit(LabelsPopoverEvent::DeSelected(label.clone()));
                }
                // 同步更新 LabelCheckListDelegate 的 checked_list
                self.label_list.update(cx, |list, cx| {
                    list.delegate_mut().set_item_checked_labels(self.selected_labels.clone(), cx);
                });
                // 发送标签ID字符串
                self.emit_labels_changed(cx);
            }
            // 移除cx.notify()调用，避免每次点击标签都重新渲染组件导致popover关闭
        }
    }

    fn selected_label(
        &mut self,
        _: &SelectedCheckLabel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_label_selection(true, cx);
    }

    fn unselected_label(
        &mut self,
        _: &UnSelectedCheckLabel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
        if let InputEvent::PressEnter { secondary: _ } = event {
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
        let colors = [
            "#ff5252", "#ff4081", "#e040fb", "#7c4dff", "#536dfe", "#448aff", "#40c4ff", "#18ffff",
            "#64ffda", "#69f0ae", "#b2ff59", "#eeff41", "#ffff00", "#ffd740", "#ffab40", "#ff6e40",
        ];
        // 使用时间戳作为随机种子来选择颜色
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as usize;
        let color = colors[timestamp % colors.len()].to_string();

        // 创建新标签模型
        let new_label = Arc::new(LabelModel {
            id: format!("label_{}_{}", timestamp, label_name.trim().replace(" ", "_")),
            name: label_name.trim().to_string(),
            color,
            item_order: 0,
            is_deleted: false,
            is_favorite: false,
            backend_type: None,
            source_id: None,
        });

        // 将新标签添加到全局标签状态
        cx.update_global::<TodoStore, _>(|store, _cx| {
            store.add_label(new_label.clone());
        });

        // 清空输入框
        self.new_label_input.update(cx, |input, cx| {
            input.set_value("".to_string(), window, cx);
        });

        // 自动选中新创建的标签
        if !self.selected_labels.iter().any(|l| l.id == new_label.id) {
            self.selected_labels.push(new_label.clone());
            // 同步更新 LabelCheckListDelegate 的 checked_list
            self.label_list.update(cx, |list, cx| {
                list.delegate_mut().set_item_checked_labels(self.selected_labels.clone(), cx);
            });
            cx.emit(LabelsPopoverEvent::Selected(new_label.clone()));
            self.emit_labels_changed(cx);
        }

        // 移除cx.notify()调用，避免创建新标签后popover关闭
    }

    // 发送标签变更事件
    fn emit_labels_changed(&self, cx: &mut Context<Self>) {
        let label_ids =
            self.selected_labels.iter().map(|label| label.id.clone()).collect::<Vec<_>>().join(";");
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
            .items_center()
            .justify_end()
            .on_action(cx.listener(Self::selected_label))
            .on_action(cx.listener(Self::unselected_label))
            .child(
                Popover::new("popover-list")
                    .p_0()
                    .text_sm()
                    .open(self.list_popover_open)
                    .on_open_change(cx.listener(move |this, open, _, cx| {
                        this.list_popover_open = *open;
                        cx.notify();
                    }))
                    .trigger(
                        Button::new("item-labels-button")
                            .small()
                            .ghost()
                            .compact()
                            .tooltip(if selected_count > 0 {
                                format!("{} labels selected", selected_count)
                            } else {
                                "Set Labels".to_string()
                            })
                            .icon(IconName::TagOutlineSymbolic)
                            .when(selected_count > 0, |this| {
                                this.label(format!("{}", selected_count))
                            }),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .p_2()
                            .child(
                                // 标签列表
                                List::new(&self.label_list).h(px(240.)),
                            )
                            .child(
                                // 分隔线
                                gpui_component::divider::Divider::horizontal().mt_2().mb_2(),
                            )
                            .child(
                                // 新建标签输入框和按钮
                                h_flex()
                                    .gap_2()
                                    .child(Input::new(&self.new_label_input).small().flex_1())
                                    .child(
                                        Button::new("create-label-button")
                                            .small()
                                            .ghost()
                                            .icon(IconName::PinSymbolic)
                                            .on_click(cx.listener(|this, _event, window, cx| {
                                                let label_name = this
                                                    .new_label_input
                                                    .read(cx)
                                                    .value()
                                                    .to_string();
                                                if !label_name.trim().is_empty() {
                                                    this.create_new_label(label_name, window, cx);
                                                }
                                            })),
                                    ),
                            ),
                    )
                    .w_64(),
            )
    }
}
