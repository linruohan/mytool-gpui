use crate::{
    CalendarStory, ColorPickerStory, DatePickerStory, ListStory, StoryContainer, TableStory,
    TodoStory,
};
use gpui::{prelude::*, *};
use gpui_component::{
    input::{InputEvent, InputState, TextInput},
    resizable::{h_resizable, resizable_panel, ResizableState},
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    v_flex, ActiveTheme as _,
};

pub struct Gallery {
    stories: Vec<(&'static str, Vec<Entity<StoryContainer>>)>,
    active_group_index: Option<usize>,
    active_index: Option<usize>,
    collapsed: bool,
    search_input: Entity<InputState>,
    sidebar_state: Entity<ResizableState>,
    _subscriptions: Vec<Subscription>,
}

impl Gallery {
    pub fn new(init_story: Option<&str>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        let _subscriptions = vec![cx.subscribe(&search_input, |this, _, e, cx| match e {
            InputEvent::Change => {
                this.active_group_index = Some(0);
                this.active_index = Some(0);
                cx.notify()
            }
            _ => {}
        })];
        let stories = vec![
            // (
            //     "Getting Started",
            //     vec![StoryContainer::panel::<WelcomeStory>(window, cx)],
            // ),
            (
                "Components",
                vec![
                    StoryContainer::panel::<CalendarStory>(window, cx),
                    StoryContainer::panel::<ColorPickerStory>(window, cx),
                    StoryContainer::panel::<DatePickerStory>(window, cx),
                    StoryContainer::panel::<TableStory>(window, cx),
                    StoryContainer::panel::<TodoStory>(window, cx),
                    StoryContainer::panel::<ListStory>(window, cx),
                ],
            ),
        ];

        let mut this = Self {
            search_input,
            stories,
            active_group_index: Some(0),
            active_index: Some(0),
            collapsed: false,
            sidebar_state: ResizableState::new(cx),
            _subscriptions,
        };

        if let Some(init_story) = init_story {
            this.set_active_story(init_story, window, cx);
        }

        this
    }

    fn set_active_story(&mut self, name: &str, window: &mut Window, cx: &mut App) {
        let name = name.to_string();
        self.search_input.update(cx, |this, cx| {
            this.set_value(&name, window, cx);
        })
    }

    pub fn view(init_story: Option<&str>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(init_story, window, cx))
    }
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self.search_input.read(cx).value().trim().to_lowercase();

        let stories: Vec<_> = self
            .stories
            .iter()
            .filter_map(|(name, items)| {
                let filtered_items: Vec<_> = items
                    .iter()
                    .filter(|story| story.read(cx).name.to_lowercase().contains(&query))
                    .cloned()
                    .collect();

                if !filtered_items.is_empty() {
                    Some((name, filtered_items))
                } else {
                    None
                }
            })
            .collect();

        let active_group = self.active_group_index.and_then(|index| stories.get(index));
        let active_story = self
            .active_index
            .and(active_group)
            .and_then(|group| group.1.get(self.active_index.unwrap()));
        let (story_name, description) =
            if let Some(story) = active_story.as_ref().map(|story| story.read(cx)) {
                (story.name.clone(), story.description.clone())
            } else {
                ("".into(), "".into())
            };

        h_resizable("gallery-container", self.sidebar_state.clone())
            .child(
                resizable_panel()
                    .size(px(255.))
                    .size_range(px(200.)..px(320.))
                    .child(
                        Sidebar::left()
                            .width(relative(1.))
                            .border_width(px(0.))
                            .collapsed(self.collapsed)
                            .header(
                                v_flex().w_full().gap_4().child(
                                    div()
                                        .bg(cx.theme().sidebar_accent)
                                        .px_1()
                                        .rounded_full()
                                        .flex_1()
                                        .mx_1()
                                        .child(
                                            TextInput::new(&self.search_input)
                                                .appearance(false)
                                                .cleanable(),
                                        ),
                                ),
                            )
                            .children(stories.clone().into_iter().enumerate().map(
                                |(group_ix, (_group_name, sub_stories))| {
                                    SidebarMenu::new().children(sub_stories.iter().enumerate().map(
                                        |(ix, story)| {
                                            SidebarMenuItem::new(story.read(cx).name.clone())
                                                .active(
                                                    self.active_group_index == Some(group_ix)
                                                        && self.active_index == Some(ix),
                                                )
                                                .on_click(cx.listener(
                                                    move |this, _: &ClickEvent, _, cx| {
                                                        this.active_group_index = Some(group_ix);
                                                        this.active_index = Some(ix);
                                                        cx.notify();
                                                    },
                                                ))
                                        },
                                    ))
                                },
                            )),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .h_full()
                    .overflow_x_hidden()
                    .child(
                        div()
                            .id("story")
                            .flex_1()
                            .overflow_y_scroll()
                            .when_some(active_story, |this, active_story| {
                                this.child(active_story.clone())
                            }),
                    )
                    .into_any_element(),
            )
    }
}
