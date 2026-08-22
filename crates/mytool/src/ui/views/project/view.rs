use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Hsla, IntoElement, ParentElement, Render,
    Styled, Subscription, Window,
};
use gpui_component::{
    ActiveTheme, Colorize, IndexPath, WindowExt,
    button::{Button, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    dialog::{DialogAction, DialogClose, DialogFooter},
    input::{Input, InputState},
    list::{ListEvent, ListState},
    v_flex,
};
use todos::entity::ProjectModel;

use crate::{
    ColorGroup, ColorGroupEvent, ColorGroupState, ProjectEvent, ProjectListDelegate,
    VisualHierarchy,
    todo_actions::{add_project, delete_project, update_project},
    todo_state::TodoStore,
};

impl EventEmitter<ProjectEvent> for ProjectsPanel {}
pub struct ProjectsPanel {
    input_esc: Entity<InputState>,
    pub project_list: Entity<ListState<ProjectListDelegate>>,
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
                let projects = {
                    let store = cx.global::<TodoStore>();
                    if !store.peek_change_mask().affects_project_list() {
                        return;
                    }
                    store.projects.clone()
                };
                cx.update_entity(&project_list_clone, |list, cx| {
                    list.delegate_mut().update_projects(projects);
                    cx.notify();
                });
                cx.notify();
            }),
            cx.subscribe(&color, |this, _, ev, _| match ev {
                ColorGroupEvent::Change(color) => {
                    this.selected_color = *color;
                    tracing::debug!("project Color changed to: {:?}", color.unwrap().to_hex());
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

        let initial_projects = cx.global::<TodoStore>().projects.clone();
        if !initial_projects.is_empty() {
            cx.update_entity(&project_list, |list, cx| {
                list.delegate_mut().update_projects(initial_projects);
                cx.notify();
            });
        }

        Self {
            input_esc,
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
            ProjectEvent::Loaded => self.sync_projects_from_store(cx),
            ProjectEvent::Added(project) => add_project(project.clone(), cx),
            ProjectEvent::Modified(project) => update_project(project.clone(), cx),
            ProjectEvent::Deleted(project) => delete_project(project.clone(), cx),
        }
    }

    fn sync_projects_from_store(&mut self, cx: &mut Context<Self>) {
        let projects = cx.global::<TodoStore>().projects.clone();
        self.project_list.update(cx, |list, cx| {
            list.delegate_mut().update_projects(projects);
            cx.notify();
        });
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
                tracing::debug!("show_label_dialog: active index: {}", index);
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
                .footer(
                    DialogFooter::new()
                        .child(
                            DialogClose::new()
                                .child(Button::new("cancel").label("Cancel").outline()),
                        )
                        .child(
                            DialogAction::new().child(Button::new("add").primary().label("Add")),
                        ),
                )
                .on_ok({
                    let view = view.clone();
                    let ori_project = ori_project.clone();
                    let input1 = name_input.clone();
                    move |_, _window: &mut Window, cx| {
                        view.update(cx, |view, cx| {
                            let project = Arc::new(ProjectModel {
                                name: input1.read(cx).value().to_string(),
                                due_date: view.project_due.clone(),
                                color: Some(
                                    view.selected_color.map(|c| c.to_hex()).unwrap_or_default(),
                                ),
                                ..ori_project.clone()
                            });
                            cx.emit(ProjectEvent::Added(project));
                            cx.notify();
                        });
                        true
                    }
                })
        });
    }
}

impl Render for ProjectsPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().w_full().gap(VisualHierarchy::spacing(4.0)).child("projects_panel")
    }
}
