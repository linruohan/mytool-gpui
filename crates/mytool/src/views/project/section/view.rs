use std::rc::Rc;

use gpui::{
    px, App, AppContext, Context, Entity, EventEmitter, Hsla, IntoElement, ParentElement,
    Render, Styled, Subscription, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants}, input::{Input, InputState}, list::{List, ListEvent, ListState}, v_flex,
    ActiveTheme,
    Colorize,
    IndexPath,
    WindowExt,
};
use todos::entity::{ProjectModel, SectionModel};

use super::{SectionEvent, SectionListDelegate};
use crate::{
    todo_actions::{add_section, delete_section, update_section}, todo_state::SectionState, ColorGroup,
    ColorGroupEvent,
    ColorGroupState,
};

impl EventEmitter<SectionEvent> for SectionsPanel {}
pub struct SectionsPanel {
    input_esc: Entity<InputState>,
    pub section_list: Entity<ListState<SectionListDelegate>>,
    project: Rc<ProjectModel>,
    pub active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
    color: Entity<ColorGroupState>,
    selected_color: Option<Hsla>,
}

impl SectionsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_esc =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter DB URL").clean_on_escape());

        let section_list =
            cx.new(|cx| ListState::new(SectionListDelegate::new(), window, cx).selectable(true));
        let color = cx.new(|cx| ColorGroupState::new(window, cx).default_value(cx.theme().primary));
        let section_list_clone = section_list.clone();
        let _subscriptions = vec![
            cx.observe_global::<SectionState>(move |_this, cx| {
                let sections = cx.global::<SectionState>().sections.clone();
                cx.update_entity(&section_list_clone, |list, cx| {
                    list.delegate_mut().update_sections(sections);
                    cx.notify();
                });
                cx.notify();
            }),
            cx.subscribe(&color, |this, _, ev, _| match ev {
                ColorGroupEvent::Change(color) => {
                    this.selected_color = *color;
                    println!("section Color changed to: {:?}", color.unwrap().to_hex());
                },
            }),
            cx.subscribe_in(&section_list, window, |this, _, ev: &ListEvent, window, cx| {
                if let ListEvent::Confirm(ix) = ev
                    && let Some(conn) = this.get_selected_section(*ix, cx)
                {
                    this.update_active_index(Some(ix.row));
                    this.input_esc.update(cx, |is, cx| {
                        is.set_value(conn.clone().name.clone(), window, cx);
                        cx.notify();
                    })
                }
            }),
        ];

        Self {
            input_esc,
            section_list,
            project: Rc::new(ProjectModel::default()),
            active_index: Some(0),
            _subscriptions,
            color,
            selected_color: None,
        }
    }

    fn get_selected_section(&self, ix: IndexPath, cx: &App) -> Option<Rc<SectionModel>> {
        self.section_list
            .read(cx)
            .delegate()
            .matched_sections
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

    pub fn handle_section_event(&mut self, event: &SectionEvent, cx: &mut Context<Self>) {
        println!("handle_section_event:");
        match event {
            SectionEvent::Added(section) => add_section(section.clone(), cx),
            SectionEvent::Modified(section) => update_section(section.clone(), cx),
            SectionEvent::Deleted(section) => delete_section(section.clone(), cx),
            _ => {},
        }
    }

    fn initialize_section_model(
        &self,
        is_edit: bool,
        _: &mut Window,
        cx: &mut App,
    ) -> SectionModel {
        self.active_index
            .filter(|_| is_edit)
            .and_then(|index| {
                println!("show_section_dialog: active index: {}", index);
                self.get_selected_section(IndexPath::new(index), cx)
            })
            .map(|section| {
                let section_ref = section.as_ref();
                SectionModel { ..section_ref.clone() }
            })
            .unwrap_or_default()
    }

    pub fn show_section_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        is_edit: bool,
    ) {
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Label Name"));
        let ori_section = self.initialize_section_model(is_edit, window, cx);
        if is_edit {
            name_input.update(cx, |is, cx| {
                is.set_value(ori_section.name.clone(), window, cx);
                cx.notify();
            })
        };

        let view = cx.entity().clone();
        let dialog_title = if is_edit { "Edit Label" } else { "New Label" };
        let button_section = if is_edit { "Save" } else { "Add" };
        let color = self.color.clone();
        window.open_dialog(cx, move |modal, _, _| {
            modal
                .title(dialog_title)
                .overlay(false)
                .keyboard(true)
                .overlay_closable(true)
                .child(
                    v_flex().gap_3().child(Input::new(&name_input)).child(ColorGroup::new(&color)),
                )
                .footer({
                    let view = view.clone();
                    let ori_section = ori_section.clone();
                    let name_input_clone = name_input.clone();
                    move |_, _, _, _cx| {
                        vec![
                            Button::new("save").primary().label(button_section).on_click({
                                let view = view.clone();
                                let ori_section = ori_section.clone();
                                let name_input_clone1 = name_input_clone.clone();
                                move |_, window, cx| {
                                    window.close_dialog(cx);
                                    view.update(cx, |view, cx| {
                                        let section = Rc::new(SectionModel {
                                            name: name_input_clone1.read(cx).value().to_string(),
                                            color: Option::from(
                                                view.selected_color.unwrap_or_default().to_hex(),
                                            ),
                                            ..ori_section.clone()
                                        });
                                        println!(
                                            "show_section_dialog: section: {:?}",
                                            section.clone()
                                        );
                                        // 根据模式发射不同事件
                                        if is_edit {
                                            cx.emit(SectionEvent::Modified(section));
                                        } else {
                                            cx.emit(SectionEvent::Added(section));
                                        }
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

    pub fn show_section_delete_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_index) = self.active_index {
            let section_some = self.get_selected_section(IndexPath::new(active_index), cx);
            if let Some(section) = section_some {
                let view = cx.entity().clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    dialog
                        .confirm()
                        .overlay(true)
                        .overlay_closable(true)
                        .child("Are you sure to delete the section?")
                        .on_ok({
                            let view = view.clone();
                            let section = section.clone();
                            move |_, window, cx| {
                                let view = view.clone();
                                let section = section.clone();
                                view.update(cx, |_view, cx| {
                                    cx.emit(SectionEvent::Deleted(section));
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
}

impl Render for SectionsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        List::new(&self.section_list)
            .p(px(2.))
            .flex_1()
            .w_full()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
    }
}
