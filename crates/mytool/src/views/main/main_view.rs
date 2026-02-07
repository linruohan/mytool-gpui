use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _,
    input::Input,
    resizable::{h_resizable, resizable_panel},
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    v_flex,
};

use crate::ComponentManager;

pub struct MainView {
    component_manager: Entity<ComponentManager>,
    collapsed: bool,
    _subscriptions: Vec<Subscription>,
}

impl MainView {
    pub fn new(init_story: Option<&str>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let component_manager = cx.new(|cx| ComponentManager::new(init_story, window, cx));
        let _subscriptions = vec![];

        Self { component_manager, collapsed: false, _subscriptions }
    }

    pub fn view(init_story: Option<&str>, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(init_story, window, cx))
    }
}

impl Render for MainView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let component_manager = self.component_manager.clone();
        let collapsed = self.collapsed;
        let search_input = component_manager.read(cx).get_search_input();
        let stories = component_manager.read(cx).stories.clone();
        let active_story = component_manager.read(cx).get_active_story(&stories);

        h_resizable("gallery-container")
            .child(
                resizable_panel().size(px(255.)).size_range(px(200.)..px(320.)).child(
                    Sidebar::new("gallery-sidebar")
                        .w(relative(1.))
                        .border_0()
                        .collapsed(collapsed)
                        .header(
                            v_flex().w_full().gap_4().child(
                                div()
                                    .bg(cx.theme().sidebar_accent)
                                    .rounded_full()
                                    .px_1()
                                    .when(cx.theme().radius.is_zero(), |this| this.rounded(px(0.)))
                                    .flex_1()
                                    .mx_1()
                                    .child(
                                        Input::new(&search_input).appearance(false).cleanable(true),
                                    ),
                            ),
                        )
                        .children(
                            // 预先计算所有需要的值，避免在闭包中捕获 cx
                            stories
                                .into_iter()
                                .enumerate()
                                .map(|(group_ix, (_group_name, sub_stories))| {
                                    let component_manager = component_manager.clone();

                                    // 预先计算每个故事的名称
                                    let story_items = sub_stories
                                        .iter()
                                        .map(|story| {
                                            let story_name = story.read(cx).name.clone();
                                            (story.clone(), story_name)
                                        })
                                        .collect::<Vec<_>>();

                                    // 创建侧边栏菜单项
                                    let menu_items = story_items
                                        .into_iter()
                                        .enumerate()
                                        .map(|(ix, (_, story_name))| {
                                            let component_manager = component_manager.clone();

                                            SidebarMenuItem::new(story_name)
                                        .active(false) // 暂时设置为 false，避免在闭包中捕获 cx
                                        .on_click(cx.listener(
                                            move |_this: &mut MainView, _: &ClickEvent, _, cx| {
                                                component_manager.update(cx, |manager, _| {
                                                    manager.set_active_indices(group_ix, ix);
                                                });
                                                cx.notify();
                                            },
                                        ))
                                        })
                                        .collect::<Vec<_>>();

                                    SidebarMenu::new().children(menu_items)
                                })
                                .collect::<Vec<_>>(),
                        ),
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
