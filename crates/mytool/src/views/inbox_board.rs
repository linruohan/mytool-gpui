use super::Board;

use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Hsla, InteractiveElement,
    InteractiveElement as _, ParentElement, Render, Styled, Window, div, px,
};

use gpui_component::{
    ActiveTheme as _, ContextModal as _, IconName, Placement,
    button::{Button, ButtonVariants as _},
    date_picker::{DatePicker, DatePickerState},
    dock::PanelControl,
    h_flex,
    input::{InputState, TextInput},
    label::Label,
    v_flex,
};

use todos::entity::ItemModel;

pub struct InboxBoard {
    focus_handle: FocusHandle,
    tasks: Vec<ItemModel>,
    drawer_placement: Option<Placement>,
}

impl InboxBoard {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    pub(crate) fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            tasks: Vec::new(),
            drawer_placement: None,
        }
    }
    pub fn tasks(&self) -> &[ItemModel] {
        &self.tasks
    }

    pub fn add_task(&mut self, task: ItemModel) {
        self.tasks.push(task);
    }
    pub fn clear_tasks(&mut self) {
        self.tasks.clear();
    }
    fn open_drawer_at(&mut self, placement: Placement, window: &mut Window, cx: &mut App) {
        println!("奥斯丁发射点法速度发生的");
        let _list_h = match placement {
            Placement::Left | Placement::Right => px(400.),
            Placement::Top | Placement::Bottom => px(160.),
        };

        let overlay = true;
        let overlay_closable = true;
        let input1 = cx.new(|cx| InputState::new(window, cx).placeholder("Your Name"));
        let _input2 = cx.new(|cx| {
            InputState::new(window, cx).placeholder("For test focus back on modal close.")
        });
        let date = cx.new(|cx| DatePickerState::new(window, cx));
        window.open_drawer_at(placement, cx, move |this, _, _cx| {
            this.overlay(overlay)
                .overlay_closable(overlay_closable)
                .size(px(400.))
                .title("Item 详情:")
                .gap_4()
                .child(TextInput::new(&input1))
                .child(DatePicker::new(&date).placeholder("Date of Birth"))
                .child(
                    Button::new("send-notification")
                        .child("Test Notification")
                        .on_click(|_, window, cx| {
                            window.push_notification("Hello this is message from Drawer.", cx)
                        }),
                )
                .footer(
                    h_flex()
                        .gap_6()
                        .items_center()
                        .child(Button::new("confirm").primary().label("确认").on_click(
                            |_, window, cx| {
                                window.close_drawer(cx);
                            },
                        ))
                        .child(
                            Button::new("cancel")
                                .label("取消")
                                .on_click(|_, window, cx| {
                                    window.close_drawer(cx);
                                }),
                        ),
                )
        });
    }
    fn close_drawer(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.drawer_placement = None;
        cx.notify();
    }
}
impl Board for InboxBoard {
    fn icon() -> IconName {
        IconName::MailboxSymbolic
    }
    fn colors() -> Vec<Hsla> {
        vec![gpui::rgb(0x99c1f1).into(), gpui::rgb(0x3584e4).into()]
    }

    fn count() -> usize {
        1
    }
    fn title() -> &'static str {
        "Inbox"
    }

    fn description() -> &'static str {
        "UI components for building fantastic desktop application by using GPUI."
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for InboxBoard {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for InboxBoard {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        v_flex()
            .flex_1()
            .h_full()
            .overflow_x_hidden()
            .child(
                h_flex()
                    .id("header")
                    .p_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .justify_between()
                    .items_start()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().text_xl().child(<InboxBoard as Board>::title()))
                            .child(
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(<InboxBoard as Board>::description()),
                            ),
                    ),
            )
            .child(Label::new("asdfasdf"))
            .child(
                Button::new("asdid")
                    .outline()
                    .label("drawer")
                    .on_click(cx.listener(|this, _, window, cx| {
                        println!("{}", "但是大声的发射点法");
                        this.open_drawer_at(Placement::Left, window, cx)
                    })),
            )
    }
}
