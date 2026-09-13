use std::sync::Arc;

use gpui::{
    App, AppContext, Context, ElementId, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, RenderOnce,
    StatefulInteractiveElement, StyleRefinement, Styled, Subscription, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    Sizable, Size, StyledExt as _,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    input::{Input, InputState, Textarea, TextareaState},
    spinner::Spinner,
    tag::Tag,
    theme::ActiveTheme,
    v_flex,
};
use gpui_kit::assets::IconName;
use todos::{entity::ItemModel, enums::item_priority::ItemPriority};
use tracing::warn;

use super::{
    AttachmentButton, AttachmentButtonState, PriorityButton, PriorityState, ProjectButton,
    ProjectButtonState, RecurrencyButton, RecurrencyButtonState, ReminderButton,
    ReminderButtonState, ScheduleButton, ScheduleButtonState, SectionButton, SectionState,
};
use crate::{
    LabelsPopoverList,
    core::state::{DBState, SaveResults, TodoStore},
    label_chip,
    todo_actions::set_item_pinned_optimistic,
    ui::theme::visual_enhancements::SemanticColors,
};

mod handlers;
mod item_state_manager;
mod labels;
mod save;
mod types;

pub use item_state_manager::{ItemStateManager, SaveItemStatus};
pub use types::ItemInfoEvent;

const CONTEXT: &str = "ItemInfo";

pub struct ItemInfoState {
    focus_handle: FocusHandle,
    /// 集中的状态管理器
    pub state_manager: ItemStateManager,
    _subscriptions: Vec<Subscription>,
    // item view
    name_input: Entity<InputState>,
    // 🔧 修复：描述框使用多行 TextareaState（auto_grow 只在该模式下可用）
    desc_input: Entity<TextareaState>,
    priority_state: Entity<PriorityState>,
    project_state: Entity<ProjectButtonState>,
    section_state: Entity<SectionState>,
    schedule_button_state: Entity<ScheduleButtonState>,
    recurrency_button_state: Entity<RecurrencyButtonState>,
    label_popover_list: Entity<LabelsPopoverList>,
    attachment_state: Entity<AttachmentButtonState>,
    reminder_state: Entity<ReminderButtonState>,
}

impl Focusable for ItemInfoState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl EventEmitter<ItemInfoEvent> for ItemInfoState {}
impl ItemInfoState {
    pub fn new(item: Arc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = item.clone();

        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("任务名称"));

        let desc_input = cx.new(|cx| {
            // 🔧 修复：auto_grow 只在多行 TextareaState 上存在，使用 TextareaState::new()
            TextareaState::new(window, cx).auto_grow(1, 8).placeholder("添加描述...")
        });
        let label_popover_list = cx.new(|cx| LabelsPopoverList::new(window, cx));

        let priority_state = cx.new(|cx| PriorityState::new(window, cx));
        let project_state = cx.new(|cx| ProjectButtonState::new(window, cx));
        let section_state = cx.new(|cx| SectionState::new(window, cx));
        let schedule_button_state = cx.new(|cx| {
            let mut state = ScheduleButtonState::new(window, cx);
            // 使用类型安全的 due_date() 方法
            if let Some(due_date) = item.due_date() {
                state.set_due_date(due_date, window, cx);
            }
            state
        });
        let recurrency_button_state = cx.new(|cx| {
            let mut state = RecurrencyButtonState::new(window, cx);
            // 如果有 due_date 且有重复设置，初始化 recurrency_button_state
            if let Some(due_date) = item.due_date() {
                state.set_due_date(due_date, window, cx);
            }
            state
        });
        let attachment_state = cx.new(|cx| AttachmentButtonState::new(item.id.clone(), window, cx));
        let reminder_state = cx.new(|cx| ReminderButtonState::new(item.id.clone(), window, cx));

        let _subscriptions = vec![
            // 🔧 修复：名称框（InputState）和描述框（TextareaState）类型不同，使用各自独立的回调
            cx.subscribe_in(&name_input, window, Self::on_name_input_event),
            cx.subscribe_in(&desc_input, window, Self::on_desc_input_event),
            cx.subscribe_in(&label_popover_list, window, Self::on_labels_event),
            cx.subscribe_in(&priority_state, window, Self::on_priority_event),
            cx.subscribe_in(&project_state, window, Self::on_project_event),
            cx.subscribe_in(&section_state, window, Self::on_section_event),
            cx.subscribe_in(&schedule_button_state, window, Self::on_schedule_event),
            cx.subscribe_in(&recurrency_button_state, window, Self::on_recurrency_event),
            cx.subscribe_in(&reminder_state, window, Self::on_reminder_event),
            cx.subscribe_in(&attachment_state, window, Self::on_attachment_event),
            // 异步保存结果写入 SaveResults 时刷新，否则失败不会把状态从 Saving 改成 Failed
            cx.observe_global::<SaveResults>(|this, cx| {
                if this.apply_save_results(cx) {
                    cx.notify();
                }
            }),
            // 订阅 TodoStore 的变化，确保 pinned 状态和其他状态变化时能够更新界面
            cx.observe_global_in::<TodoStore>(window, move |this, _window, cx| {
                if this.state_manager.skip_next_update {
                    tracing::debug!("ItemInfoState: skip TodoStore overwrite while saving");
                    if this.state_manager.save_status != SaveItemStatus::Saving {
                        this.state_manager.skip_next_update = false;
                    }
                    // 仍要处理 temp_ → 真实 ID，否则后续保存会按临时 ID UPDATE 失败
                    if this.apply_persisted_id_mapping(cx) {
                        cx.notify();
                    }
                    return;
                }

                let mask = *cx.global::<TodoStore>().peek_change_mask();
                if !mask.affects_item_editor() {
                    return;
                }

                let mut item_changed = false;
                if mask.items_changed {
                    let current_id = this.state_manager.item.id.clone();
                    let updated_item = cx.global::<TodoStore>().get_item(&current_id);

                    if let Some(updated_item) = updated_item {
                        if !std::sync::Arc::ptr_eq(&this.state_manager.item, &updated_item)
                            && !this.state_manager.item.display_eq(&updated_item)
                        {
                            this.state_manager.item = updated_item;
                            item_changed = true;
                        }
                    } else if current_id.starts_with("temp_") {
                        item_changed = this.apply_persisted_id_mapping(cx);
                    }
                }

                if item_changed
                    || mask.projects_changed
                    || mask.sections_changed
                    || mask.labels_changed
                {
                    cx.notify();
                }
            }),
        ];
        let mut this = Self {
            focus_handle: cx.focus_handle(),
            state_manager: ItemStateManager::new(item.clone()),
            _subscriptions,
            name_input,
            desc_input,
            priority_state,
            project_state,
            section_state,
            schedule_button_state,
            recurrency_button_state,
            label_popover_list,
            attachment_state,
            reminder_state,
        };
        this.set_item(item, window, cx);
        this
    }

    /// 将仍停留在 temp_ ID 上的编辑器同步到数据库真实 ID
    fn apply_persisted_id_mapping(&mut self, cx: &mut Context<Self>) -> bool {
        let current_id = self.state_manager.item.id.clone();
        if !current_id.starts_with("temp_") {
            return false;
        }

        let real_item = {
            let store = cx.global::<TodoStore>();
            store.get_real_id(&current_id).and_then(|real_id| store.get_item(real_id))
        };
        let Some(real_item) = real_item else {
            return false;
        };

        tracing::debug!(
            "ItemInfoState: detected ID change from {} to {} via mapping",
            current_id,
            real_item.id
        );

        self.state_manager.item = real_item.clone();
        let new_item_id = real_item.id.clone();
        self.attachment_state.update(cx, |state, cx| {
            state.update_item_id(new_item_id.clone(), cx);
        });
        self.reminder_state.update(cx, |state, cx| {
            state.update_item_id(new_item_id, cx);
        });
        true
    }

    /// 消费异步保存结果，避免在 render 里改状态。
    fn apply_save_results(&mut self, cx: &mut Context<Self>) -> bool {
        let item_id = self.state_manager.item.id.clone();
        let Some(save_success) = cx.global::<SaveResults>().take_result(&item_id) else {
            return false;
        };

        self.state_manager.skip_next_update = false;
        if save_success {
            tracing::debug!("save succeeded for {}", item_id);
            self.state_manager.mark_clean();
            self.state_manager.update_original();
            self.state_manager.save_status = SaveItemStatus::Succeeded;
        } else {
            warn!("save failed for {}, keeping dirty", item_id);
            self.state_manager.mark_dirty();
            self.state_manager.save_status = SaveItemStatus::Failed;
        }
        true
    }

    // set item of item_info
    pub fn set_item(&mut self, item: Arc<ItemModel>, window: &mut Window, cx: &mut Context<Self>) {
        self.set_item_internal(item, window, cx, true);
    }

    /// 更新 item 但不重新加载标签（用于避免覆盖用户的标签更改）
    pub fn update_item_without_reloading_labels(
        &mut self,
        item: Arc<ItemModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_item_internal(item, window, cx, false);
    }

    /// 内部方法：设置 item，可选择是否重新加载标签
    fn set_item_internal(
        &mut self,
        item: Arc<ItemModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
        reload_labels: bool,
    ) {
        let previous_id = self.state_manager.item.id.clone();
        let item_id_changed = previous_id != item.id;

        self.state_manager = ItemStateManager::new(item.clone());

        self.name_input.update(cx, |this, cx| {
            this.set_value(item.content.clone(), window, cx);
        });
        self.desc_input.update(cx, |this, cx| {
            this.set_value(item.description.clone().unwrap_or_default(), window, cx);
        });
        self.priority_state.update(cx, |this, cx| {
            if let Some(priority) = item.priority {
                this.set_priority(ItemPriority::from_i32(priority), window, cx);
            }
        });

        // 根据当前项目解析分区列表，避免整表 clone
        let item_section_id = item.section_id.clone();
        let (project_ok, filtered_sections, section_in_store) = {
            let store = cx.global::<TodoStore>();
            let project_ok =
                item.project_id.as_ref().is_some_and(|id| store.get_project(id).is_some());
            let filtered_sections = item
                .project_id
                .as_ref()
                .filter(|_| project_ok)
                .map(|id| store.sections_for_project(id));
            let section_in_store =
                item_section_id.as_ref().and_then(|id| store.get_section(id)).is_some();
            (project_ok, filtered_sections, section_in_store)
        };

        self.project_state.update(cx, |this, cx| {
            if project_ok {
                this.set_project(item.project_id.clone(), window, cx);
            }
        });

        self.section_state.update(cx, |section_state, cx| {
            if item.project_id.is_some() {
                if let Some(filtered) = filtered_sections {
                    if let Some(section_id) = &item_section_id
                        && !filtered.iter().any(|s| &s.id == section_id)
                    {
                        self.state_manager.set_section_id(None);
                    }
                    section_state.set_sections(Some(filtered), window, cx);
                }
            } else {
                section_state.set_sections(None, window, cx);
            }

            if let Some(section_id) = &item_section_id {
                let valid = match &section_state.sections {
                    Some(ss) => ss.iter().any(|s| &s.id == section_id),
                    None => section_in_store,
                };
                if valid {
                    section_state.set_section(Some(section_id.clone()), window, cx);
                }
            } else {
                section_state.set_section(None, window, cx);
            }
        });

        // 使用类型安全的 due_date() 方法
        self.schedule_button_state.update(cx, |this, cx| {
            if let Some(due_date) = item.due_date() {
                this.set_due_date(due_date, window, cx);
                return;
            }
            this.set_due_date(todos::DueDate::default(), window, cx);
        });

        // 更新 recurrency_button_state
        self.recurrency_button_state.update(cx, |this, cx| {
            if let Some(due_date) = item.due_date() {
                this.set_due_date(due_date, window, cx);
            } else {
                this.set_due_date(todos::DueDate::default(), window, cx);
            }
        });

        if item_id_changed || reload_labels {
            self.load_related_records(item.id.clone(), reload_labels || item_id_changed, cx);
        }
    }

    fn load_related_records(&self, item_id: String, reload_labels: bool, cx: &mut Context<Self>) {
        if item_id.is_empty() {
            return;
        }

        let label_popover_list = self.label_popover_list.clone();
        let attachment_state = self.attachment_state.clone();
        let reminder_state = self.reminder_state.clone();
        let this_entity = cx.entity();
        let db_state = cx.global::<DBState>().clone();

        cx.spawn(async move |_this, cx| {
            let loaded = db_state
                .spawn_store_op(move |store| async move {
                    let labels = if reload_labels {
                        Some(store.get_labels_by_item(&item_id).await.unwrap_or_default())
                    } else {
                        None
                    };
                    let attachments =
                        store.get_attachments_by_item(&item_id).await.unwrap_or_default();
                    let reminders = store.get_reminders_by_item(&item_id).await.unwrap_or_default();
                    Ok((labels, attachments, reminders))
                })
                .await;

            let (labels, attachments, reminders) = match loaded {
                Ok(Ok(data)) => data,
                Ok(Err(e)) => {
                    tracing::error!("Failed to load item related data: {:?}", e);
                    return;
                },
                Err(e) => {
                    tracing::error!("Item related data task panicked: {:?}", e);
                    return;
                },
            };

            if let Some(item_labels) = labels {
                let label_ids_str =
                    item_labels.iter().map(|label| label.id.as_str()).collect::<Vec<_>>().join(";");
                cx.update_entity(&label_popover_list, |popover_list, cx| {
                    popover_list.set_item_checked_label_id_async(label_ids_str, cx);
                });
            }

            cx.update_entity(&attachment_state, |state: &mut AttachmentButtonState, cx| {
                state.set_attachments(attachments.into_iter().map(Arc::new).collect(), cx);
            });
            cx.update_entity(&reminder_state, |state: &mut ReminderButtonState, cx| {
                state.set_reminders(reminders.into_iter().map(Arc::new).collect(), cx);
            });
            cx.update_entity(&this_entity, |_item_info_state, cx| {
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for ItemInfoState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let selected_labels = self.selected_labels(cx);
        let has_sections = self
            .section_state
            .read(cx)
            .sections
            .as_ref()
            .is_some_and(|sections| !sections.is_empty());

        let colors = SemanticColors::from_theme(cx);
        let pinned_color = if self.state_manager.item.pinned {
            colors.status_pinned
        } else {
            cx.theme().muted_foreground
        };
        let has_date = !self.schedule_button_state.read(cx).due_date.date.is_empty();
        let label_popover = self.label_popover_list.clone();
        let muted = cx.theme().muted_foreground;

        div()
            .id("item-info-body")
            .w_full()
            .on_mouse_down(gpui::MouseButton::Left, |_event, _window, cx| {
                cx.stop_propagation();
            })
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(
                        h_flex()
                            .w_full()
                            .items_start()
                            .gap_2()
                            .child(
                                Checkbox::new(format!(
                                    "item-checked-{}",
                                    self.state_manager.item.id
                                ))
                                .checked(self.state_manager.item.checked)
                                .on_click(cx.listener(Self::toggle_finished)),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_1()
                                    .child(
                                        h_flex()
                                            .w_full()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                Input::new(&self.name_input)
                                                    .appearance(false)
                                                    .focus_bordered(false)
                                                    .flex_1()
                                                    .min_w_0(),
                                            )
                                            .child(
                                                Button::new("item-pin")
                                                    .small()
                                                    .ghost()
                                                    .compact()
                                                    .icon(IconName::PinSymbolic)
                                                    .text_color(pinned_color)
                                                    .tooltip(if self.state_manager.item.pinned {
                                                        "取消置顶"
                                                    } else {
                                                        "置顶任务"
                                                    })
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        let item = this.state_manager.item.clone();
                                                        let pinned = item.pinned;
                                                        set_item_pinned_optimistic(
                                                            item, !pinned, cx,
                                                        );
                                                    })),
                                            )
                                            .when(
                                                self.state_manager.save_status
                                                    != SaveItemStatus::Idle,
                                                |this| match self.state_manager.save_status {
                                                    SaveItemStatus::Saving => this.child(
                                                        Spinner::new()
                                                            .small()
                                                            .color(cx.theme().warning),
                                                    ),
                                                    SaveItemStatus::Succeeded => this.child(
                                                        Tag::success().small().child("已保存"),
                                                    ),
                                                    SaveItemStatus::Failed => this.child(
                                                        Tag::danger().small().child("保存失败"),
                                                    ),
                                                    _ => this,
                                                },
                                            )
                                            .child(
                                                Button::new("collapse-item")
                                                    .small()
                                                    .ghost()
                                                    .compact()
                                                    .icon(IconName::ChevronUp)
                                                    .tooltip("收起 (Enter)")
                                                    .on_click(cx.listener(|_, _, _, cx| {
                                                        cx.emit(ItemInfoEvent::Collapse());
                                                    })),
                                            ),
                                    )
                                    .child(
                                        Textarea::new(&self.desc_input)
                                            .appearance(false)
                                            .bordered(false)
                                            .text_sm()
                                            .text_color(muted),
                                    )
                                    .when(!selected_labels.is_empty(), |this| {
                                        this.child(h_flex().gap_1().flex_wrap().children(
                                            selected_labels.iter().map(|label| {
                                                let popover = label_popover.clone();
                                                let chip_id =
                                                    format!("edit-label-chip-{}", label.id);
                                                div()
                                                    .id(chip_id)
                                                    .cursor_pointer()
                                                    .on_click(move |_, window, cx| {
                                                        popover.update(cx, |this, cx| {
                                                            this.list_popover_open = true;
                                                            this.label_list.update(
                                                                cx,
                                                                |list, cx| {
                                                                    list.focus(window, cx);
                                                                },
                                                            );
                                                            cx.notify();
                                                        });
                                                    })
                                                    .child(label_chip(
                                                        label.name.clone(),
                                                        &label.color,
                                                    ))
                                            }),
                                        ))
                                    }),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .pl(px(28.))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .min_w_0()
                                    .child(ScheduleButton::new(&self.schedule_button_state))
                                    .when(has_date, |this| {
                                        this.child(RecurrencyButton::new(
                                            &self.recurrency_button_state,
                                        ))
                                    })
                                    .child(
                                        ProjectButton::new(&self.project_state)
                                            .max_w(px(140.))
                                            .flex_shrink_0(),
                                    )
                                    .when(has_sections, |this| {
                                        this.child(
                                            SectionButton::new(&self.section_state)
                                                .max_w(px(120.))
                                                .flex_shrink_0(),
                                        )
                                    }),
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .flex_shrink_0()
                                    .when(!self.state_manager.item.is_subtask(), |this| {
                                        this.child(
                                            Button::new("add-subtask")
                                                .small()
                                                .ghost()
                                                .compact()
                                                .icon(IconName::Plus)
                                                .tooltip("添加子任务")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.add_subtask(window, cx);
                                                })),
                                        )
                                    })
                                    .child(AttachmentButton::new(&self.attachment_state))
                                    .child(self.label_popover_list.clone())
                                    .child(PriorityButton::new(&self.priority_state))
                                    .child(ReminderButton::new(&self.reminder_state)),
                            ),
                    ),
            )
    }
}

#[derive(IntoElement)]
pub struct ItemInfo {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    state: Entity<ItemInfoState>,
}

impl Sizable for ItemInfo {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
impl Focusable for ItemInfo {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl Styled for ItemInfo {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ItemInfo {
    pub fn new(state: &Entity<ItemInfoState>) -> Self {
        Self {
            id: ("item-info", state.entity_id()).into(),
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for ItemInfo {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            // 移除 track_focus，让子组件（输入框）自己管理焦点
            .w_full()
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}
