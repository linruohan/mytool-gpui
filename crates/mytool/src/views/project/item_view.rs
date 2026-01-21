use std::rc::Rc;

use gpui::{
    App, AppContext, BorrowAppContext, Context, Entity, EventEmitter, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Subscription, Window, div,
};
use gpui_component::{
    ActiveTheme, IconName, IndexPath, WindowExt,
    button::{Button, ButtonVariants},
    h_flex,
    menu::{DropdownMenu, PopupMenuItem},
    v_flex,
};
use todos::entity::{ItemModel, ProjectModel};

use crate::{
    ItemEvent, ItemInfo, ItemInfoEvent, ItemInfoState, ItemRow, ItemRowState,
    todo_actions::{
        add_project_item, delete_project_item, load_project_items, update_project_item,
    },
    todo_state::ProjectState,
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
    project: Rc<ProjectModel>,
    pub active_index: Option<usize>,
    item_rows: Vec<Entity<ItemRowState>>,
    item_info: Entity<ItemInfoState>,
    _subscriptions: Vec<Subscription>,
}

impl ProjectItemsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let item = Rc::new(ItemModel::default());
        let item_info = cx.new(|cx| ItemInfoState::new(item.clone(), window, cx));
        let item_rows = vec![];

        let _subscriptions =
            vec![cx.observe_global_in::<ProjectState>(window, move |this, window, cx| {
                // 单一数据源：只从 ProjectState.items 构建 UI，保证顺序稳定，active_index 不会错位
                let state_items = cx.global::<ProjectState>().items.clone();
                this.item_rows = state_items
                    .iter()
                    .map(|item| cx.new(|cx| ItemRowState::new(item.clone(), window, cx)))
                    .collect();

                // 保护 active_index，避免删除/刷新后越界导致后续 unwrap 崩溃
                if let Some(ix) = this.active_index {
                    if ix >= this.item_rows.len() {
                        this.active_index = this.item_rows.is_empty().then_some(0).or(None);
                    }
                } else if !this.item_rows.is_empty() {
                    this.active_index = Some(0);
                }
                cx.notify();
            })];

        Self {
            active_index: Some(0),
            item_rows,
            item_info,
            _subscriptions,
            project: Rc::new(ProjectModel::default()),
        }
    }

    pub fn set_project(&mut self, project: Rc<ProjectModel>, cx: &mut Context<Self>) {
        self.project = project.clone();
        // 切换项目时，先清空当前 items，避免新项目为空时仍短暂显示旧项目
        cx.update_global::<ProjectState, _>(|state, _| {
            state.items.clear();
        });
        // 重置选中项，避免沿用上一个项目的索引
        self.active_index = Some(0);
        load_project_items(project.clone(), cx);
    }

    pub(crate) fn get_selected_item(&self, ix: IndexPath, cx: &App) -> Option<Rc<ItemModel>> {
        let item_list = cx.global::<ProjectState>().items.clone();
        item_list.get(ix.row).cloned()
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
                                            "https://github.com/linruohan/gpui-component",
                                        )
                                        .separator()
                                        .item(PopupMenuItem::new("Edit item").on_click(
                                            window.listener_for(&view, |this, _, window, cx| {
                                                if let Some(model) =
                                                    this.active_index.map(IndexPath::new).and_then(
                                                        |index| this.get_selected_item(index, cx),
                                                    )
                                                {
                                                    // 已有选中项 => 编辑模式
                                                    this.show_model(model, true, window, cx);
                                                } else {
                                                    // 没有选中项 => 新建模式
                                                    this.show_model(
                                                        Rc::new(ItemModel::default()),
                                                        false,
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
                                                        if let Some(index) = this.active_index
                                                            && let Some(item) = this
                                                                .get_selected_item(
                                                                    IndexPath::new(index),
                                                                    cx,
                                                                )
                                                        {
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
            .child(v_flex().children(self.item_rows.iter().map(|item| ItemRow::new(&item))))
    }
}
