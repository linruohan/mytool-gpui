use std::rc::Rc;

use gpui::{
    App, AppContext, ClickEvent, Context, Entity, EventEmitter, Hsla, IntoElement, ParentElement,
    Render, Styled, Subscription, WeakEntity, Window,
};
use gpui_component::{
    ActiveTheme, Colorize, IconName, IndexPath, WindowExt,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    input::{Input, InputState},
    list::{ListEvent, ListState},
    menu::{DropdownMenu, PopupMenuItem},
    sidebar::{SidebarMenu, SidebarMenuItem},
    v_flex,
};
use todos::entity::ProjectModel;

use crate::{
    ColorGroup, ColorGroupEvent, ColorGroupState, ProjectEvent, ProjectListDelegate, play_ogg_file,
    service::load_projects,
    todo_state::{DBState, ProjectState},
};

impl EventEmitter<ProjectEvent> for ProjectsPanel {}
pub struct ProjectsPanel {
    input_esc: Entity<InputState>,
    pub project_list: Entity<ListState<ProjectListDelegate>>,
    is_loading: bool,
    project_due: Option<String>,
    color: Entity<ColorGroupState>,
    selected_color: Option<Hsla>,
    pub active_index: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl ProjectsPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_esc =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter DB URL").clean_on_escape());

        let project_list = cx.new(|cx| ListState::new(ProjectListDelegate::new(), window, cx));
        let color = cx.new(|cx| ColorGroupState::new(window, cx).default_value(cx.theme().primary));
        let project_list_clone = project_list.clone();
        let _subscriptions = vec![
            cx.observe_global::<ProjectState>(move |_this, cx| {
                let projects = cx.global::<ProjectState>().projects.clone();
                let _ = cx.update_entity(&project_list_clone, |list, cx| {
                    list.delegate_mut().update_projects(projects);
                    cx.notify();
                });
                cx.notify();
            }),
            cx.subscribe(&color, |this, _, ev, _| match ev {
                ColorGroupEvent::Change(color) => {
                    this.selected_color = *color;
                    println!("project Color changed to: {:?}", color.unwrap().to_hex());
                },
            }),
            cx.subscribe_in(&project_list, window, |this, _, ev: &ListEvent, window, cx| {
                if let ListEvent::Confirm(ix) = ev
                    && let Some(conn) = this.get_selected_project(*ix, cx)
                {
                    this.input_esc.update(cx, |is, cx| {
                        is.set_value(conn.clone().name.clone(), window, cx);
                        cx.notify();
                    })
                }
            }),
        ];

        Self {
            input_esc,
            is_loading: false,
            project_list,
            active_index: Some(0),
            project_due: None,
            color,
            selected_color: None,
            _subscriptions,
        }
    }

    pub(crate) fn get_selected_project(&self, ix: IndexPath, cx: &App) -> Option<Rc<ProjectModel>> {
        self.project_list
            .read(cx)
            .delegate()
            .matched_projects
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

    pub fn handle_project_event(&mut self, event: &ProjectEvent, cx: &mut Context<Self>) {
        match event {
            ProjectEvent::Loaded => {
                self.update_projects(cx);
            },
            ProjectEvent::Added(project) => {
                println!("handle_project_event:");
                self.add_project(cx, project.clone())
            },
            ProjectEvent::Modified(project) => self.mod_project(cx, project.clone()),
            ProjectEvent::Deleted(project) => self.del_project(cx, project.clone()),
        }
    }

    fn initialize_project_model(
        &self,
        is_edit: bool,
        _: &mut Window,
        cx: &mut App,
    ) -> ProjectModel {
        self.active_index
            .filter(|_| is_edit)
            .and_then(|index| {
                println!("show_label_dialog: active index: {}", index);
                self.get_selected_project(IndexPath::new(index), &cx)
            })
            .map(|label| {
                let item_ref = label.as_ref();
                ProjectModel { ..item_ref.clone() }
            })
            .unwrap_or_default()
    }

    pub fn open_project_dialog(
        &mut self,
        _model: Rc<ProjectModel>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Project Name"));
        let now = chrono::Local::now().naive_local().date();
        let project_due = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
            picker.set_date(now, window, cx);
            picker
        });
        let color = self.color.clone();
        let is_edit = false;
        let ori_project = self.initialize_project_model(is_edit, window, cx);
        let _ = cx.subscribe(&project_due, |this, _, ev, _| match ev {
            DatePickerEvent::Change(date) => {
                this.project_due = date.format("%Y-%m-%d").map(|s| s.to_string());
            },
        });

        let view = cx.entity().clone();

        window.open_dialog(cx, move |modal, _, _| {
            modal
                .title("New Project")
                .overlay(false)
                .keyboard(true)
                .overlay_closable(true)
                .child(
                    v_flex()
                        .gap_3()
                        .child(Input::new(&name_input))
                        .child(ColorGroup::new(&color))
                        .child(DatePicker::new(&project_due).placeholder("DueDate of Project")),
                )
                .footer({
                    let view = view.clone();
                    let ori_project = ori_project.clone();
                    let input1 = name_input.clone();
                    move |_, _, _, _cx| {
                        vec![
                            Button::new("add").primary().label("Add").on_click({
                                let view = view.clone();
                                let ori_project = ori_project.clone();
                                let input1 = input1.clone();
                                move |_, window, cx| {
                                    window.close_dialog(cx);
                                    view.update(cx, |view, cx| {
                                        let project = Rc::new(ProjectModel {
                                            name: input1.read(cx).value().to_string(),
                                            due_date: view.project_due.clone(),
                                            color: Some(
                                                view.selected_color.unwrap_or_default().to_hex(),
                                            ),
                                            ..ori_project.clone()
                                        });
                                        cx.emit(ProjectEvent::Added(project));
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

    // 更新projects
    fn update_projects(&mut self, cx: &mut Context<Self>) {
        if !self.is_loading {
            return;
        }
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this, cx| {
            let db = db.lock().await;
            let projects = load_projects(db.clone()).await;
            let rc_projects: Vec<Rc<ProjectModel>> =
                projects.iter().map(|pro| Rc::new(pro.clone())).collect();

            this.update(cx, |this, cx| {
                this.project_list.update(cx, |list, cx| {
                    list.delegate_mut().update_projects(rc_projects);
                    cx.notify();
                });

                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub fn add_project(&mut self, cx: &mut Context<Self>, project: Rc<ProjectModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ProjectsPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::add_project(project.clone(), db.clone()).await;
            println!("add_project {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.update_projects(cx);
    }

    pub fn mod_project(&mut self, cx: &mut Context<Self>, project: Rc<ProjectModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ProjectsPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::mod_project(project.clone(), db.clone()).await;
            println!("mod_project {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.update_projects(cx);
    }

    pub fn del_project(&mut self, cx: &mut Context<Self>, project: Rc<ProjectModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ProjectsPanel>, cx| {
            let db = db.lock().await;
            let ret = crate::service::del_project(project.clone(), db.clone()).await;
            println!("mod_project {:?}", ret);
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
        self.update_projects(cx);
    }
}

impl Render for ProjectsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let projects: Vec<_> = self.project_list.read(cx).delegate()._projects.clone();
        let view = cx.entity();
        v_flex().w_full().gap_4().child(
            // 添加项目按钮：
            SidebarMenu::new()
                .child(SidebarMenuItem::new("On This Computer                     ➕").on_click(
                    cx.listener(move |this, _, window: &mut Window, cx| {
                        // let projects = projects.read(cx);
                        println!("project_panel: {}", "add projects");
                        play_ogg_file("assets/sounds/success.ogg");
                        let default_model = Rc::new(ProjectModel::default());
                        this.open_project_dialog(default_model, window, cx);
                        cx.notify();
                    }),
                ))
                .children(projects.iter().enumerate().map(|(ix, story)| {
                    SidebarMenuItem::new(story.name.clone())
                        .active(self.active_index == Some(ix))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.active_index = Some(ix);
                            cx.notify();
                        }))
                        .suffix(
                            Button::new("project-popup-menu")
                                .label("label")
                                .icon(IconName::EllipsisVertical)
                                .dropdown_menu({
                                    let view = view.clone();
                                    move |this, window, _cx| {
                                        this.link(
                                            "About",
                                            "https://github.com/longbridge/gpui-component",
                                        )
                                        .separator()
                                        .item(PopupMenuItem::new("Edit project").on_click(
                                            window.listener_for(&view, |this, _, window, cx| {
                                                if let Some(model) = this
                                                    .active_index
                                                    .map(IndexPath::new)
                                                    .and_then(|index| {
                                                        this.get_selected_project(index, cx)
                                                    })
                                                {
                                                    this.open_project_dialog(model, window, cx);
                                                }
                                                cx.notify();
                                            }),
                                        ))
                                        .separator()
                                        .item(
                                            PopupMenuItem::new("Delete project").on_click(
                                                window.listener_for(
                                                    &view,
                                                    |this, _, _window, cx| {
                                                        let index = this.active_index.unwrap();
                                                        let project_some = this
                                                            .get_selected_project(
                                                                IndexPath::new(index),
                                                                cx,
                                                            );
                                                        if let Some(project) = project_some {
                                                            this.del_project(cx, project.clone());
                                                        }
                                                        cx.notify();
                                                    },
                                                ),
                                            ),
                                        )
                                    }
                                }),
                        )
                })),
        )
    }
}
