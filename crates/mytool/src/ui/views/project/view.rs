use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Hsla, IntoElement, ParentElement, Render,
    Styled, Subscription, WeakEntity, Window,
};
use gpui_component::{
    ActiveTheme, Colorize, IndexPath, WindowExt,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    input::{Input, InputState},
    list::{ListEvent, ListState},
    v_flex,
};
use todos::entity::ProjectModel;

use crate::{
    ColorGroup, ColorGroupEvent, ColorGroupState, ProjectEvent, ProjectListDelegate,
    VisualHierarchy,
    state_service::load_projects,
    todo_state::{DBState, TodoStore},
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
            cx.observe_global::<TodoStore>(move |_this, cx| {
                let projects = cx.global::<TodoStore>().projects.clone();
                cx.update_entity(&project_list_clone, |list, cx| {
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

    pub(crate) fn get_selected_project(
        &self,
        ix: IndexPath,
        cx: &App,
    ) -> Option<Arc<ProjectModel>> {
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
                self.get_selected_project(IndexPath::new(index), cx)
            })
            .map(|label| {
                let item_ref = label.as_ref();
                ProjectModel { ..item_ref.clone() }
            })
            .unwrap_or_default()
    }

    pub fn open_project_dialog(
        &mut self,
        _model: Arc<ProjectModel>,
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
                        .gap(VisualHierarchy::spacing(3.0))
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
                                        let project = Arc::new(ProjectModel {
                                            name: input1.read(cx).value().to_string(),
                                            due_date: view.project_due.clone(),
                                            color: Some(
                                                view.selected_color
                                                    .map(|c| c.to_hex())
                                                    .unwrap_or_default(),
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
            let projects = load_projects(db.clone()).await;
            let rc_projects: Vec<Arc<ProjectModel>> =
                projects.iter().map(|pro| Arc::new(pro.clone())).collect();

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

    pub fn add_project(&mut self, cx: &mut Context<Self>, project: Arc<ProjectModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ProjectsPanel>, cx| {
            let ret = crate::state_service::add_project(project.clone(), db.clone()).await;
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

    pub fn mod_project(&mut self, cx: &mut Context<Self>, project: Arc<ProjectModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ProjectsPanel>, cx| {
            let ret = crate::state_service::mod_project(project.clone(), db.clone()).await;
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

    pub fn del_project(&mut self, cx: &mut Context<Self>, project: Arc<ProjectModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this: WeakEntity<ProjectsPanel>, cx| {
            let ret = crate::state_service::del_project(project.clone(), db.clone()).await;
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
        let _projects: Vec<_> = self.project_list.read(cx).delegate()._projects.clone();
        let _view = cx.entity();
        v_flex().w_full().gap(VisualHierarchy::spacing(4.0)).child("projects_panel")
    }
}
