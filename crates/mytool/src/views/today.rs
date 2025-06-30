use gpui::{
    div, App, AppContext, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, Styled,
    Subscription, Window,
};

use gpui_component::{
    button::Button,
    checkbox::Checkbox,
    h_flex,
    list::{List, ListDelegate, ListEvent},
    v_flex, ActiveTheme, Sizable,
};

use super::project::{random_project, Project, ProjectListDelegate, SelectedProject};
pub struct TodayView {
    focus_handle: FocusHandle,
    project_list: Entity<List<ProjectListDelegate>>,
    selected_project: Option<Project>,
    _subscriptions: Vec<Subscription>,
}

impl crate::Mytool for TodayView {
    fn title() -> &'static str {
        "List"
    }

    fn description() -> &'static str {
        "A list displays a series of items."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl TodayView {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let projects = (0..3).map(|_| random_project()).collect::<Vec<Project>>();

        let delegate = ProjectListDelegate {
            matched_projects: projects.clone(),
            projects,
            selected_index: Some(0),
            confirmed_index: None,
            query: "".to_string(),
            loading: false,
            eof: false,
        };

        let project_list = cx.new(|cx| List::new(delegate, window, cx));
        // project_list.update(cx, |list, cx| {
        //     list.set_selected_index(Some(3), cx);
        // });
        let _subscriptions =
            vec![
                cx.subscribe(&project_list, |_, _, ev: &ListEvent, _| match ev {
                    ListEvent::Select(ix) => {
                        println!("List Selected: {:?}", ix);
                    }
                    ListEvent::Confirm(ix) => {
                        println!("List Confirmed: {:?}", ix);
                    }
                    ListEvent::Cancel => {
                        println!("List Cancelled");
                    }
                }),
            ];

        // Spawn a background to random refresh the list
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                this.project_list.update(cx, |picker, _| {
                    picker
                        .delegate_mut()
                        .projects
                        .iter_mut()
                        .for_each(|project| {
                            project.random_update();
                        });
                });
                cx.notify();
            })
            .ok();
        })
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            project_list,
            selected_project: None,
            _subscriptions,
        }
    }

    fn selected_project(&mut self, _: &SelectedProject, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.project_list.read(cx);
        if let Some(project) = picker.delegate().selected_project() {
            self.selected_project = Some(project);
        }
    }
}


impl Focusable for TodayView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TodayView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_project))
            .size_full()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        Button::new("scroll-top")
                            .child("Scroll to Top")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.project_list.update(cx, |list, cx| {
                                    list.scroll_to_item(0, window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-bottom")
                            .child("Scroll to Bottom")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.project_list.update(cx, |list, cx| {
                                    list.scroll_to_item(
                                        list.delegate().items_count(cx) - 1,
                                        window,
                                        cx,
                                    );
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-to-selected")
                            .child("Scroll to Selected")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.project_list.update(cx, |list, cx| {
                                    if let Some(selected) = list.selected_index() {
                                        list.scroll_to_item(selected, window, cx);
                                    }
                                })
                            })),
                    )
                    .child(
                        Checkbox::new("loading")
                            .label("Loading")
                            .checked(self.project_list.read(cx).delegate().loading)
                            .on_click(cx.listener(|this, check: &bool, _, cx| {
                                this.project_list.update(cx, |this, cx| {
                                    this.delegate_mut().loading = *check;
                                    cx.notify();
                                })
                            })),
                    ),
            )
            .child(
                div()   
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(self.project_list.clone()),
            )
    }
}
