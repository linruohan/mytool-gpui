use std::sync::Arc;

use gpui::{
    App, AppContext, BorrowAppContext, Context, Entity, EventEmitter, Hsla, IntoElement,
    ParentElement, Render, Styled, Subscription, Window,
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
                tracing::debug!("handle_project_event:");
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

    // 更新projects
    fn update_projects(&mut self, cx: &mut Context<Self>) {
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |this, cx| {
            let projects = load_projects((*db).clone()).await;
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

        let project_for_save = project.clone();

        let temp_id = format!("temp_project_{}", uuid::Uuid::new_v4());
        let temp_project =
            Arc::new(ProjectModel { id: temp_id.clone(), ..project.as_ref().clone() });
        cx.update_global::<TodoStore, _>(|todo_store, _| {
            todo_store.add_project(temp_project.clone());
        });

        // 使用独立 SQLite 连接绕过连接池竞争（避免 ConnectionAcquire(Timeout））
        std::thread::spawn(move || {
            let db_path = "db.sqlite";
            let absolute_path = if std::path::Path::new(db_path).is_absolute() {
                db_path.to_string()
            } else {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.join(db_path)))
                    .unwrap_or_else(|| std::path::PathBuf::from(db_path))
                    .to_string_lossy()
                    .to_string()
            };

            let save_result =
                todos::services::project_service::ProjectService::insert_project_direct(
                    &absolute_path,
                    project_for_save.as_ref().clone(),
                );

            if save_result.is_err() {
                tracing::error!("❌ [add_project] DB 保存失败: {:?}", save_result.err());
            }
        });
    }

    pub fn mod_project(&mut self, cx: &mut Context<Self>, project: Arc<ProjectModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();

        let db_state = cx.global::<crate::todo_state::DBState>().clone();
        let project_for_save = project.clone();

        cx.update_global::<TodoStore, _>(|todo_store, _| {
            todo_store.update_project(project.clone());
        });

        // 使用独立线程执行 DB 操作（避免 GPUI Future 被取消）
        std::thread::spawn(move || {
            let save_result = crate::core::tokio_runtime::run_db_operation(async move {
                db_state.wait_for_store_ready(Some(std::time::Duration::from_secs(5))).await?;
                let store = db_state.get_store_async().await;
                crate::core::utils::retry::retry_async_todo(
                    move |_attempt| {
                        let store = store.clone();
                        let proj = project_for_save.clone();
                        async move { crate::state_service::mod_project_with_store(proj, store).await }
                    },
                    crate::core::utils::retry::RetryConfig::for_db_operation(),
                )
                .await
            });

            if let Err(e) = &save_result {
                tracing::error!("❌ [mod_project] DB 更新失败: {:?}", e);
            }
        });
    }

    pub fn del_project(&mut self, cx: &mut Context<Self>, project: Arc<ProjectModel>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        cx.notify();

        let db_state = cx.global::<crate::todo_state::DBState>().clone();
        let project_for_delete = project.clone();
        let project_id = project.id.clone();

        cx.update_global::<TodoStore, _>(|todo_store, _| {
            todo_store.remove_project(&project_id);
        });

        // 使用独立线程执行 DB 操作（避免 GPUI Future 被取消）
        std::thread::spawn(move || {
            let save_result = crate::core::tokio_runtime::run_db_operation(async move {
                db_state.wait_for_store_ready(Some(std::time::Duration::from_secs(5))).await?;
                let store = db_state.get_store_async().await;
                crate::core::utils::retry::retry_async_todo(
                    move |_attempt| {
                        let store = store.clone();
                        let proj = project_for_delete.clone();
                        async move { crate::state_service::del_project_with_store(proj, store).await }
                    },
                    crate::core::utils::retry::RetryConfig::for_db_operation(),
                )
                .await
            });

            if let Err(e) = &save_result {
                tracing::error!("❌ [del_project] DB 删除失败: {:?}", e);
            }
        });
    }
}

impl Render for ProjectsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _projects: Vec<_> = self.project_list.read(cx).delegate()._projects.clone();
        let _view = cx.entity();
        v_flex().w_full().gap(VisualHierarchy::spacing(4.0)).child("projects_panel")
    }
}
