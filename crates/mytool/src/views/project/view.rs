use crate::{load_projects, play_ogg_file, DBState, ProjectEvent, ProjectListDelegate};
use gpui::{
    App, AppContext, ClickEvent, Context, Entity, EventEmitter, IntoElement, ParentElement, Render,
    Styled, Subscription, WeakEntity, Window,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::date_picker::{DatePicker, DatePickerEvent, DatePickerState};
use gpui_component::input::{Input, InputState};
use gpui_component::list::{List, ListEvent};
use gpui_component::sidebar::{SidebarMenu, SidebarMenuItem};
use gpui_component::switch::Switch;
use gpui_component::{v_flex, ContextModal, IndexPath, Sizable};
use std::rc::Rc;
use todos::entity::ProjectModel;

impl EventEmitter<ProjectEvent> for ProjectListPanel {}
pub struct ProjectListPanel {
    input_esc: Entity<InputState>,
    project_list: Entity<List<ProjectListDelegate>>,
    project: Rc<ProjectModel>,
    project_due: Option<String>,
    active_index: Option<usize>,
    is_connected: bool,
    is_loading: bool,
    collapsed: bool,
    _subscriptions: Vec<Subscription>,
}

impl ProjectListPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_esc = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter DB URL")
                .clean_on_escape()
        });

        let project_list = cx.new(|cx| List::new(ProjectListDelegate::new(), window, cx));

        let _subscriptions = vec![cx.subscribe_in(
            &project_list,
            window,
            |this, _, ev: &ListEvent, window, cx| {
                if let ListEvent::Confirm(ix) = ev
                    && let Some(conn) = this.get_selected_project(*ix, cx)
                {
                    this.input_esc.update(cx, |is, cx| {
                        is.set_value(conn.clone().name.clone(), window, cx);
                        cx.notify();
                    })
                }
            },
        )];

        let project_list_clone = project_list.clone();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |_view, cx| {
            let db = db.lock().await;
            let projects = load_projects(db.clone()).await;
            let rc_projects: Vec<Rc<ProjectModel>> =
                projects.iter().map(|pro| Rc::new(pro.clone())).collect();
            let _ = cx
                .update_entity(&project_list_clone, |list, cx| {
                    list.delegate_mut().update_projects(rc_projects);
                    cx.notify();
                })
                .ok();
        })
          .detach();
        Self {
            input_esc,
            is_connected: false,
            is_loading: false,
            project_list,
            project: Rc::new(ProjectModel::default()),
            project_due: None,
            active_index: None,
            collapsed: false,
            _subscriptions,
        }
    }
    fn get_selected_project(&self, ix: IndexPath, cx: &App) -> Option<Rc<ProjectModel>> {
        self.project_list
            .read(cx)
            .delegate()
            .matched_projects
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
    pub fn handle_project_event(&mut self, event: &ProjectEvent, cx: &mut Context<Self>) {
        match event {
            ProjectEvent::Added(project) => self.add_project(cx, project.clone()),
            ProjectEvent::Modified(project) => self.mod_project(cx, project.clone()),
            ProjectEvent::Deleted(project) => self.del_project(cx, project.clone()),
        }
    }
    fn add_project_model(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Project Name"));
        let _input2 = cx.new(|cx| -> InputState {
            InputState::new(window, cx).placeholder("For test focus back on modal close.")
        });
        let now = chrono::Local::now().naive_local().date();
        let project_due = cx.new(|cx| {
            let mut picker = DatePickerState::new(window, cx).disabled_matcher(vec![0, 6]);
            picker.set_date(now, window, cx);
            picker
        });
        let _ = cx.subscribe(&project_due, |this, _, ev, _| match ev {
            DatePickerEvent::Change(date) => {
                this.project_due = date.format("%Y-%m-%d").map(|s| s.to_string());
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
                        .child(DatePicker::new(&project_due).placeholder("DueDate of Project")),
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
                                    view.update(cx, |view, cx| {
                                        let project = ProjectModel {
                                            name: input1.read(cx).value().to_string(),
                                            due_date: view.project_due.clone(),
                                            ..Default::default()
                                        };
                                        cx.emit(ProjectEvent::Added(project.into()));
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

    pub fn add_project(&mut self, cx: &mut Context<Self>, project: Rc<ProjectModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        cx.spawn(async move |this: WeakEntity<ProjectListPanel>, cx| {
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.emit(ProjectEvent::Added(project.clone()));
                cx.notify();
            })
                .ok();
        })
          .detach();
    }
    pub fn mod_project(&mut self, cx: &mut Context<Self>, project: Rc<ProjectModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        cx.spawn(async move |this: WeakEntity<ProjectListPanel>, cx| {
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.emit(ProjectEvent::Modified(project.clone()));
                cx.notify();
            })
                .ok();
        })
          .detach();
    }
    pub fn del_project(&mut self, cx: &mut Context<Self>, project: Rc<ProjectModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();
        cx.spawn(async move |this: WeakEntity<ProjectListPanel>, cx| {
            this.update(cx, |this, cx| {
                this.is_loading = false;
                cx.emit(ProjectEvent::Deleted(project.clone()));
                cx.notify();
            })
                .ok();
        })
          .detach();
    }
}

impl Render for ProjectListPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let projects: Vec<_> = self.project_list.read(cx).delegate()._projects.clone();
        v_flex()
            .w_full()
            .gap_4()
            .child(
                // 添加项目按钮：
                SidebarMenu::new().child(
                    SidebarMenuItem::new("On This Computer                     ➕").on_click(
                        cx.listener(move |this, _, window: &mut Window, cx| {
                            // let projects = projects.read(cx);
                            println!("{}", "add projects");
                            play_ogg_file("assets/sounds/success.ogg");
                            this.add_project_model(window, cx);
                            cx.notify();
                        }),
                    ),
                ).children(projects.iter().enumerate().map(|(ix, story)| {
                    SidebarMenuItem::new(story.name.clone())
                        .active(self.active_index == Some(ix))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.active_index = Some(ix);
                            cx.notify();
                        }))
                        .suffix(Switch::new("dark-mode").checked(true).xsmall())
                })))
    }
}
