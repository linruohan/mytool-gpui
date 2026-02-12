use gpui::{
    Action, App, Context, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, StyleRefinement, Styled, Window, div, px,
};
use gpui_component::{
    Icon, IconName, Sizable, Size, StyledExt,
    button::{Button, ButtonVariants},
    menu::DropdownMenu,
    v_flex,
};
// Generic dropdown state
pub struct DropdownState<T: Clone + PartialEq + 'static + Send> {
    focus_handle: FocusHandle,
    pub selected: Option<T>,
}

#[derive(Clone)]
pub enum DropdownEvent<T: Clone + PartialEq + 'static + Send> {
    Selected(T),
}

impl<T: Clone + PartialEq + 'static + Send> Action for DropdownEvent<T> {
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new((*self).clone())
    }

    fn partial_eq(&self, _other: &dyn Action) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "DropdownEvent"
    }

    fn name_for_type() -> &'static str {
        "DropdownEvent"
    }

    fn build(_: serde_json::Value) -> Result<Box<dyn Action>, anyhow::Error> {
        Err(anyhow::anyhow!("Cannot build DropdownEvent from JSON"))
    }
}

impl<T: Clone + PartialEq + 'static + Send> EventEmitter<DropdownEvent<T>> for DropdownState<T> {}

impl<T: Clone + PartialEq + 'static + Send> Focusable for DropdownState<T> {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl<T: Clone + PartialEq + 'static + Send> DropdownState<T> {
    pub fn new(_window: &mut Window, cx: &mut Context<impl std::any::Any>) -> Self {
        Self { focus_handle: cx.focus_handle(), selected: None }
    }

    pub fn selected(&self) -> Option<T> {
        self.selected.clone()
    }

    pub fn set_selected(
        &mut self,
        selected: Option<T>,
        _: &mut Window,
        cx: &mut Context<impl std::any::Any>,
    ) {
        self.selected = selected;
        cx.notify();
    }
}

impl<T: Clone + PartialEq + 'static + Send> Render for DropdownState<T> {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

// Helper trait for items that can be displayed in dropdown
pub trait DropdownItem: Clone + PartialEq + 'static + Send {
    fn display_name(&self) -> String;
    fn id(&self) -> String;
}

/// 简化 Button on_click 模式的通用辅助函数
pub fn make_click_handler<T, F>(
    view: gpui::Entity<T>,
    handler: F,
) -> impl Fn(gpui::ClickEvent, &mut Window, &mut App) + 'static
where
    T: 'static,
    F: Fn(&mut T, &mut Context<T>) + 'static,
{
    move |_event, _window, cx| {
        cx.update_entity(&view, handler);
    }
}

// Base trait for button components
pub trait BaseButtonComponent: Sized {
    fn id(&self) -> gpui::ElementId;
    fn style(&mut self) -> &mut StyleRefinement;
    fn size(&self) -> Size;
    fn focus_handle(&self, cx: &App) -> FocusHandle;
}

/// Helper macro to create standard button wrapper components
#[macro_export]
macro_rules! create_button_wrapper {
    ($button_name:ident, $state_name:ident, $button_id:expr) => {
        #[derive(gpui::IntoElement)]
        pub struct $button_name {
            id: gpui::ElementId,
            style: gpui::StyleRefinement,
            size: gpui_component::Size,
            state: gpui::Entity<$state_name>,
        }

        impl gpui_component::Sizable for $button_name {
            fn with_size(mut self, size: impl Into<gpui_component::Size>) -> Self {
                self.size = size.into();
                self
            }
        }

        impl gpui::Focusable for $button_name {
            fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
                self.state.focus_handle(cx)
            }
        }

        impl gpui::Styled for $button_name {
            fn style(&mut self) -> &mut gpui::StyleRefinement {
                &mut self.style
            }
        }

        impl $button_name {
            pub fn new(state: &gpui::Entity<$state_name>) -> Self {
                Self {
                    id: ($button_id, state.entity_id()).into(),
                    state: state.clone(),
                    size: gpui_component::Size::default(),
                    style: gpui::StyleRefinement::default(),
                }
            }
        }

        impl gpui::RenderOnce for $button_name {
            fn render(
                self,
                _window: &mut gpui::Window,
                cx: &mut gpui::App,
            ) -> impl gpui::IntoElement {
                use gpui::{InteractiveElement, Styled};
                use gpui_component::{StyleSized, StyledExt};

                gpui::div()
                    .id(self.id.clone())
                    .track_focus(&gpui::Focusable::focus_handle(&self, cx).tab_stop(true))
                    .flex_none()
                    .relative()
                    .input_text_size(self.size)
                    .refine_style(&self.style)
                    .child(self.state.clone())
            }
        }
    };
}

/// 通用的下拉按钮状态trait
pub trait DropdownButtonStateTrait<T: Clone + PartialEq + 'static + Send>:
    EventEmitter<Self::EventType> + Focusable + Sized
{
    type EventType: Clone;

    /// 创建新的状态实例
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self;

    /// 获取内部的DropdownState引用
    fn inner(&self) -> &DropdownState<T>;

    /// 获取内部的DropdownState可变引用
    fn inner_mut(&mut self) -> &mut DropdownState<T>;

    /// 获取当前选中值
    fn selected(&self) -> Option<T> {
        self.inner().selected()
    }

    /// 设置选中值
    fn set_selected(&mut self, selected: Option<T>, window: &mut Window, cx: &mut Context<Self>) {
        self.inner_mut().set_selected(selected, window, cx);
    }

    /// 处理选择动作
    fn on_action_select(
        &mut self,
        action: &DropdownEvent<T>,
        window: &mut Window,
        cx: &mut Context<Self>,
    );

    /// 获取按钮的ID
    fn button_id(&self) -> &'static str;

    /// 获取按钮的tooltip文本
    fn tooltip_text(&self) -> &'static str;

    /// 获取选中项的显示名称
    fn selected_display_name(&self, cx: &mut Context<Self>) -> String;

    /// 获取下拉菜单选项列表
    fn menu_options(&self, cx: &mut Context<Self>) -> Vec<(String, T)>;

    /// 获取按钮图标（可选）
    fn button_icon(&self) -> Option<IconName> {
        None
    }

    /// 获取按钮图标颜色（可选）
    fn button_icon_color(&self) -> Option<u32> {
        None
    }

    /// 获取按钮的最小宽度
    fn min_width(&self) -> f32 {
        100.0
    }
}

/// 通用的Render实现辅助函数
pub fn render_dropdown_button<T, S>(
    state: &mut S,
    _window: &mut Window,
    cx: &mut Context<S>,
) -> impl gpui::IntoElement
where
    S: DropdownButtonStateTrait<T>,
    T: Clone + PartialEq + 'static + Send,
{
    let selected_name = state.selected_display_name(cx);
    let button_id = state.button_id();
    let tooltip = state.tooltip_text();
    let icon = state.button_icon();
    let min_width = state.min_width();
    let options = state.menu_options(cx);

    let mut button = Button::new(button_id)
        .small()
        .ghost()
        .compact()
        .outline()
        .tooltip(tooltip)
        .label(SharedString::from(selected_name));

    if let Some(icon_name) = icon {
        button = button.icon(
            Icon::new(icon_name)
                .text_color(gpui::rgb(state.button_icon_color().unwrap_or(0x333333))),
        );
    }

    v_flex().on_action(cx.listener(S::on_action_select)).child(button.dropdown_menu_with_anchor(
        gpui::Corner::TopLeft,
        move |this: gpui_component::menu::PopupMenu, _, _| {
            let mut this = this.scrollable(true).max_h(px(400.));

            for (display_name, value) in options.clone() {
                this = this.menu(
                    SharedString::from(display_name),
                    Box::new(DropdownEvent::Selected(value)),
                );
            }

            this.min_w(px(min_width))
        },
    ))
}

// Generic dropdown button component
#[derive(IntoElement)]
pub struct DropdownButton<T: Clone + PartialEq + 'static + Send> {
    id: gpui::ElementId,
    style: StyleRefinement,
    size: Size,
    state: gpui::Entity<DropdownState<T>>,
}

impl<T: Clone + PartialEq + 'static + Send> gpui_component::Sizable for DropdownButton<T> {
    fn with_size(mut self, size: impl Into<gpui_component::Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl<T: Clone + PartialEq + 'static + Send> Focusable for DropdownButton<T> {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.focus_handle(cx)
    }
}

impl<T: Clone + PartialEq + 'static + Send> gpui::Styled for DropdownButton<T> {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl<T: Clone + PartialEq + 'static + Send> DropdownButton<T> {
    pub fn new(state: &gpui::Entity<DropdownState<T>>) -> Self {
        Self {
            id: ("dropdown-button", state.entity_id()).into(),
            state: state.clone(),
            size: gpui_component::Size::default(),
            style: StyleRefinement::default(),
        }
    }
}

impl<T: Clone + PartialEq + 'static + Send> gpui::RenderOnce for DropdownButton<T> {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        use gpui::InteractiveElement;
        use gpui_component::StyleSized;

        div()
            .id(self.id.clone())
            .track_focus(&self.focus_handle(cx).tab_stop(true))
            .flex_none()
            .relative()
            .input_text_size(self.size)
            .refine_style(&self.style)
            .child(self.state.clone())
    }
}

/// 宏用于简化状态结构体的基础实现
#[macro_export]
macro_rules! impl_button_state_base {
    ($state_name:ident, $event_type:ty) => {
        impl gpui::EventEmitter<$event_type> for $state_name {}

        impl gpui::Focusable for $state_name {
            fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
                self.focus_handle.clone()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dropdown_event_clone() {
        let event = DropdownEvent::Selected("test".to_string());
        let cloned = event.clone();
        assert_eq!("test", match cloned {
            DropdownEvent::Selected(s) => s,
        });
    }

    #[test]
    fn test_dropdown_item_trait() {
        #[derive(Clone, PartialEq, Debug)]
        struct TestItem {
            id: String,
            name: String,
        }

        impl DropdownItem for TestItem {
            fn display_name(&self) -> String {
                self.name.clone()
            }

            fn id(&self) -> String {
                self.id.clone()
            }
        }

        let item = TestItem { id: "1".to_string(), name: "Test".to_string() };
        assert_eq!("Test", item.display_name());
        assert_eq!("1", item.id());
    }
}
