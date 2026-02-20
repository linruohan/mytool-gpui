use gpui::{prelude::*, *};
use gpui_component::input::InputState;

use crate::StoryContainer;

pub struct ComponentManager {
    pub stories: Vec<(&'static str, Vec<Entity<StoryContainer>>)>,
    active_group_index: Option<usize>,
    active_index: Option<usize>,
    search_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl ComponentManager {
    pub fn new(init_story: Option<&str>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        let _subscriptions = vec![cx.subscribe(&search_input, |this, _, e, cx| {
            if let gpui_component::input::InputEvent::Change = e {
                this.active_group_index = Some(0);
                this.active_index = Some(0);
                cx.notify()
            }
        })];
        let stories = vec![("Components", vec![
            StoryContainer::panel::<crate::WelcomeStory>(window, cx),
            StoryContainer::panel::<crate::CalendarStory>(window, cx),
            StoryContainer::panel::<crate::TodoStory>(window, cx),
            StoryContainer::panel::<crate::ListStory>(window, cx),
        ])];

        let mut this = Self {
            search_input,
            stories,
            active_group_index: Some(0),
            active_index: Some(0),
            _subscriptions,
        };

        if let Some(init_story) = init_story {
            this.set_active_story(init_story, window, cx);
        }

        this
    }

    pub fn set_active_story(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        let name = name.to_string();
        self.search_input.update(cx, |this, cx| {
            this.set_value(&name, window, cx);
        })
    }

    pub fn get_search_input(&self) -> Entity<InputState> {
        self.search_input.clone()
    }

    pub fn get_stories(&self, cx: &mut App) -> Vec<(&'static str, Vec<Entity<StoryContainer>>)> {
        let query = self.search_input.read(cx).value().trim().to_lowercase();

        self.stories
            .iter()
            .filter_map(|(name, items)| {
                let filtered_items: Vec<_> = items
                    .iter()
                    .filter(|story| story.read(cx).name.to_lowercase().contains(&query))
                    .cloned()
                    .collect();

                if !filtered_items.is_empty() { Some((*name, filtered_items)) } else { None }
            })
            .collect()
    }

    pub fn get_active_group_index(&self) -> Option<usize> {
        self.active_group_index
    }

    pub fn get_active_index(&self) -> Option<usize> {
        self.active_index
    }

    pub fn set_active_indices(&mut self, group_index: usize, item_index: usize) {
        self.active_group_index = Some(group_index);
        self.active_index = Some(item_index);
    }

    pub fn get_active_story(
        &self,
        stories: &Vec<(&'static str, Vec<Entity<StoryContainer>>)>,
    ) -> Option<Entity<StoryContainer>> {
        let active_group = self.active_group_index.and_then(|index| stories.get(index));
        active_group.and_then(|group| group.1.get(self.active_index?)).cloned()
    }
}
