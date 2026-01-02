use gpui::{
    actions, div, prelude::FluentBuilder as _, Action, App, AppContext, Bounds, Context, Corner, ElementId,
    Entity, EventEmitter, FocusHandle, Focusable, Hsla, InteractiveElement as _, IntoElement,
    ParentElement, Pixels, Render, RenderOnce, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription, Window,
};
use gpui_component::{
    h_flex, input::{InputEvent, InputState}, tooltip::Tooltip, v_flex, ActiveTheme as _, Colorize as _, Icon,
    Sizable,
    Size,
    StyleSized,
};
use serde::Deserialize;

#[derive(Clone, Action, PartialEq, Eq, Deserialize)]
#[action(namespace = color_group, no_json)]
pub struct ColorGroupConfirm {
    /// Is confirm with secondary.
    pub secondary: bool,
}

actions!(color_group, [ColorGroupCancel, SelectUp, SelectDown, SelectLeft, SelectRight]);
const CONTEXT: &'static str = "ColorPickerGroup";
use todos::utils::Util;

use crate::section;

/// Events emitted by the [`ColorGroup`].
#[derive(Clone)]
pub enum ColorGroupEvent {
    Change(Option<Hsla>),
}

fn color_palettes() -> Vec<Vec<Hsla>> {
    use itertools::Itertools as _;
    let colors = Util::default().get_colors();
    let color_list = colors
        .keys()
        .sorted()
        .map(|k| Hsla::from(gpui::rgb(Util::default().get_color_u32_by_key(k.to_string()))))
        .collect::<Vec<_>>();
    color_list.chunks(10).map(|chunk| chunk.to_vec()).collect()
}

/// State of the [`ColorGroup`].
pub struct ColorGroupState {
    focus_handle: FocusHandle,
    value: Option<Hsla>,
    hovered_color: Option<Hsla>,
    state: Entity<InputState>,
    open: bool,
    bounds: Bounds<Pixels>,
    _subscriptions: Vec<Subscription>,
}

impl ColorGroupState {
    /// Create a new [`ColorGroupState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| InputState::new(window, cx));

        let _subscriptions = vec![cx.subscribe_in(
            &state,
            window,
            |this, state, ev: &InputEvent, window, cx| match ev {
                InputEvent::Change => {
                    let value = state.read(cx).value();
                    if let Ok(color) = Hsla::parse_hex(value.as_str()) {
                        this.value = Some(color);
                        this.hovered_color = Some(color);
                    }
                },
                InputEvent::PressEnter { .. } => {
                    let val = this.state.read(cx).value();
                    if let Ok(color) = Hsla::parse_hex(&val) {
                        this.open = false;
                        this.update_value(Some(color), true, window, cx);
                    }
                },
                _ => {},
            },
        )];

        Self {
            focus_handle: cx.focus_handle(),
            value: None,
            hovered_color: None,
            state,
            open: false,
            bounds: Bounds::default(),
            _subscriptions,
        }
    }

    /// Set default color value.
    pub fn default_value(mut self, value: impl Into<Hsla>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set current color value.
    pub fn set_value(
        &mut self,
        value: impl Into<Hsla>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.update_value(Some(value.into()), false, window, cx)
    }

    /// Get current color value.
    pub fn value(&self) -> Option<Hsla> {
        self.value
    }

    fn on_confirm(&mut self, _: &ColorGroupConfirm, _: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }

    fn update_value(
        &mut self,
        value: Option<Hsla>,
        emit: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.value = value;
        self.hovered_color = value;
        self.state.update(cx, |view, cx| {
            if let Some(value) = value {
                view.set_value(value.to_hex(), window, cx);
            } else {
                view.set_value("", window, cx);
            }
        });
        if emit {
            cx.emit(ColorGroupEvent::Change(value));
        }
        cx.notify();
    }
}

impl EventEmitter<ColorGroupEvent> for ColorGroupState {}

impl Render for ColorGroupState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.state.clone()
    }
}

impl Focusable for ColorGroupState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// A color picker element.
#[derive(IntoElement)]
pub struct ColorGroup {
    id: ElementId,
    style: StyleRefinement,
    state: Entity<ColorGroupState>,
    featured_colors: Option<Vec<Hsla>>,
    label: Option<SharedString>,
    icon: Option<Icon>,
    size: Size,
    anchor: Corner,
}

impl ColorGroup {
    /// Create a new color picker element with the given [`ColorGroupState`].
    pub fn new(state: &Entity<ColorGroupState>) -> Self {
        Self {
            id: ("color-picker", state.entity_id()).into(),
            style: StyleRefinement::default(),
            state: state.clone(),
            featured_colors: None,
            size: Size::Small,
            label: None,
            icon: None,
            anchor: Corner::TopLeft,
        }
    }

    /// Set the featured colors to be displayed in the color picker.
    ///
    /// This is used to display a set of colors that the user can quickly select from,
    /// for example provided user's last used colors.
    pub fn featured_colors(mut self, colors: Vec<Hsla>) -> Self {
        self.featured_colors = Some(colors);
        self
    }

    /// Set the icon to the color picker button.
    ///
    /// If this is set the color picker button will display the icon.
    /// Else it will display the square color of the current value.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the label to be displayed above the color picker.
    ///
    /// Default is `None`.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the anchor corner of the color picker.
    ///
    /// Default is `Corner::TopLeft`.
    pub fn anchor(mut self, anchor: Corner) -> Self {
        self.anchor = anchor;
        self
    }

    fn render_colors(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let _featured_colors = self.featured_colors.clone().unwrap_or(vec![
            cx.theme().red,
            cx.theme().red_light,
            cx.theme().blue,
            cx.theme().blue_light,
            cx.theme().green,
            cx.theme().green_light,
            cx.theme().yellow,
            cx.theme().yellow_light,
            cx.theme().cyan,
            cx.theme().cyan_light,
            cx.theme().magenta,
            cx.theme().magenta_light,
        ]);

        let state = self.state.clone();

        v_flex().gap_3().items_center().child(v_flex().gap_1().children(
            color_palettes().iter().map(|sub_colors| {
                h_flex().gap_1().children(sub_colors.iter().enumerate().map(|(_ix, color)| {
                    // self.render_item(*color, true, window, cx)
                    let color = *color;
                    div()
                        .id(SharedString::from(format!("color-{}", color.to_hex())))
                        .h_5()
                        .w_5()
                        .rounded_full()
                        .bg(color)
                        .border_1()
                        .border_color(color.darken(0.1))
                        .hover(|this| {
                            this.border_color(color.darken(0.3)).bg(color.lighten(0.1)).shadow_xs()
                        })
                        .active(|this| {
                            this.border_color(color.lighten(0.5)).bg(color.darken(0.2)).border_2()
                        })
                        .on_click(window.listener_for(&state, move |state, _, window, cx| {
                            state.update_value(Some(color), true, window, cx);
                            state.open = false;
                            cx.notify();
                        }))
                }))
            }),
        ))
    }
}

impl Sizable for ColorGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Focusable for ColorGroup {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus_handle.clone()
    }
}

impl Styled for ColorGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ColorGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let _bounds = state.bounds;
        let display_title: SharedString =
            if let Some(value) = state.value { value.to_hex() } else { "".to_string() }.into();

        let _is_focused = state.focus_handle.is_focused(window);
        let focus_handle = state.focus_handle.clone().tab_stop(true);

        div()
            .id(self.id.clone())
            .key_context(CONTEXT)
            .track_focus(&focus_handle)
            .on_action(window.listener_for(&self.state, ColorGroupState::on_confirm))
            .child(
                section("")
                    .child(
                        div()
                            .id("color-picker-square")
                            .bg(cx.theme().background)
                            .border_1()
                            .shadow_xs()
                            .rounded_full()
                            .overflow_hidden()
                            .size_with(self.size)
                            .when_some(state.value, |this, value| {
                                this.bg(value)
                                    .border_color(value.darken(0.3))
                                    .when(state.open, |this| this.border_2())
                            })
                            .when(!display_title.is_empty(), |this| {
                                this.tooltip(move |_, cx| {
                                    cx.new(|_| Tooltip::new(display_title.clone())).into()
                                })
                            }),
                    )
                    .child(self.render_colors(window, cx)),
            )
    }
}
