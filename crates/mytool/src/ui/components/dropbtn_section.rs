use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, Focusable, ParentElement, Render, Styled,
    Subscription, Window,
};
use gpui_component::{
    IndexPath, Sizable,
    searchable_list::SearchableVec,
    select::{Select, SelectEvent, SelectState},
};
use todos::entity::SectionModel;

use crate::{create_button_wrapper, todo_state::TodoStore, ui::components::drop_btn::NamedOption};

#[derive(Clone)]
pub enum SectionEvent {
    Selected(String),
}

pub struct SectionState {
    select: Entity<SelectState<SearchableVec<NamedOption>>>,
    pub sections: Option<Vec<Arc<SectionModel>>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<SectionEvent> for SectionState {}

impl Focusable for SectionState {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.select.focus_handle(cx)
    }
}

impl Render for SectionState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        self.sync_items(window, cx);
        Select::new(&self.select)
            .small()
            .appearance(false)
            .placeholder("无分区")
            .search_placeholder("搜索分区")
            .w_full()
    }
}

impl SectionState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let select = cx.new(|cx| {
            SelectState::new(
                SearchableVec::new(Self::options(None, cx)),
                Some(IndexPath::default()),
                window,
                cx,
            )
            .searchable(true)
        });

        let _subscriptions = vec![cx.subscribe(
            &select,
            |_, _, event: &SelectEvent<SearchableVec<NamedOption>>, cx| {
                if let SelectEvent::Confirm(Some(section_id)) = event {
                    cx.emit(SectionEvent::Selected(section_id.clone()));
                }
            },
        )];

        Self { select, sections: None, _subscriptions }
    }

    fn options(sections: Option<&[Arc<SectionModel>]>, cx: &App) -> Vec<NamedOption> {
        let mut options = vec![NamedOption::new(String::new(), "无分区")];
        match sections {
            Some(sections) => {
                for section in sections {
                    options.push(NamedOption::new(section.id.clone(), section.name.clone()));
                }
            },
            None => {
                for section in cx.global::<TodoStore>().sections.iter() {
                    options.push(NamedOption::new(section.id.clone(), section.name.clone()));
                }
            },
        }
        options
    }

    fn sync_items(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let items = Self::options(self.sections.as_deref(), cx);
        self.select.update(cx, |select, cx| {
            select.set_items(SearchableVec::new(items), window, cx);
        });
    }

    pub fn section_id(&self, cx: &App) -> Option<String> {
        self.select.read(cx).selected_value().cloned()
    }

    pub fn set_section(
        &mut self,
        section_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = section_id.unwrap_or_default();
        self.select.update(cx, |select, cx| {
            select.set_selected_value(&id, window, cx);
        });
    }

    pub fn set_sections(
        &mut self,
        sections: Option<Vec<Arc<SectionModel>>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sections = sections;
        let items = Self::options(self.sections.as_deref(), cx);
        self.select.update(cx, |select, cx| {
            select.set_items(SearchableVec::new(items), window, cx);
            select.set_selected_value(&String::new(), window, cx);
        });
        cx.notify();
    }
}

create_button_wrapper!(SectionButton, SectionState, "item-section");
