use std::rc::Rc;

use gpui::{
    prelude::FluentBuilder as _, App, AppContext, ClickEvent, Context, ElementId, Entity, FocusHandle,
    Focusable, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    Window,
};
use gpui_component::{button::Button, checkbox::Checkbox, h_flex, ActiveTheme, Selectable};
use todos::entity::ItemModel;

pub struct ItemRow {
    focus_handle: FocusHandle,
    selected: bool,
    checked: bool,
    item: Rc<ItemModel>,
    is_project_view: bool,
    project_id: Option<String>,
    section_id: Option<String>,
    parent_id: Option<String>,
}

impl ItemRow {
    pub fn new(
        id: impl Into<ElementId>,
        item: Rc<ItemModel>,
        is_project_view: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let _id: ElementId = id.into();
        let item = item.clone();
        Self {
            focus_handle: cx.focus_handle(),
            selected: false,
            checked: false,
            item: item.clone(),
            is_project_view,
            project_id: item.project_id.clone(),
            section_id: item.section_id.clone(),
            parent_id: item.parent_id.clone(),
        }
    }

    pub fn view(
        item: Rc<ItemModel>,
        is_project_view: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let item = item.clone();
        cx.new(|cx| Self::new(item.id.clone(), item.clone(), is_project_view, window, cx))
    }

    /// Set ListItem as the selected item style.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_checked(&mut self, checked: &bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.checked = *checked;
        println!("on clicked: {}", self.checked);
        cx.notify();
    }
}
impl Focusable for ItemRow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Selectable for ItemRow {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Render for ItemRow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .justify_between()
            .when(self.selected, |this| this.hover(|this| this.bg(cx.theme().list_hover)))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_x_1()
                    .child(
                        Checkbox::new("is-checked")
                            .checked(self.checked)
                            .on_click(cx.listener(Self::on_checked)),
                    )
                    .child(Button::new("11").label(self.item.content.clone()).on_click(
                        cx.listener(move |this, _event: &ClickEvent, _window, cx| {
                            println!("item clicked: {}", this.item.content);
                            cx.notify();
                        }),
                    )),
            )
    }
}
