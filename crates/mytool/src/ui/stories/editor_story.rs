use gpui::*;
use gpui_component::{
    ActiveTheme, IconName, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState, TabSize},
    switch::Switch,
    v_flex,
};

use crate::Mytool;

pub struct EditorStory {
    editor: Entity<InputState>,
    focus_handle: FocusHandle,
    line_number: bool,
    indent_guides: bool,
    soft_wrap: bool,
    show_whitespaces: bool,
}

impl EditorStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("rust")
                .line_number(true)
                .indent_guides(true)
                .tab_size(TabSize { tab_size: 4, hard_tabs: false })
                .soft_wrap(false)
                .default_value(
                    r#"// 欢迎使用 Editor Story!
// 这是一个简单的代码编辑器示例

fn main() {
    println!("Hello, World!");

    // 尝试编辑这段代码
    let numbers = vec![1, 2, 3, 4, 5];
    let sum: i32 = numbers.iter().sum();
    println!("Sum: {}", sum);
}

struct Example {
    name: String,
    value: i32,
}

impl Example {
    fn new(name: &str, value: i32) -> Self {
        Self {
            name: name.to_string(),
            value,
        }
    }
}
"#,
                )
                .placeholder("Enter your code here...")
        });

        let focus_handle = editor.focus_handle(cx);
        window.defer(cx, move |window, cx| {
            focus_handle.focus(window, cx);
        });

        Self {
            editor,
            focus_handle: cx.focus_handle(),
            line_number: true,
            indent_guides: true,
            soft_wrap: false,
            show_whitespaces: false,
        }
    }
}

impl Mytool for EditorStory {
    fn title() -> &'static str {
        "Editor 编辑器"
    }

    fn description() -> &'static str {
        "A simple code editor example with syntax highlighting"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for EditorStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EditorStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let position = self.editor.read(cx).cursor_position();
        let cursor = self.editor.read(cx).cursor();

        v_flex()
            .id("editor-story")
            .size_full()
            .gap_2()
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .p_2()
                    .bg(cx.theme().background)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Switch::new("line-number")
                            .label("Line Number")
                            .checked(self.line_number)
                            .on_click(cx.listener(|this, checked: &bool, window, cx| {
                                this.line_number = *checked;
                                this.editor.update(cx, |state, cx| {
                                    state.set_line_number(this.line_number, window, cx);
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Switch::new("indent-guides")
                            .label("Indent Guides")
                            .checked(self.indent_guides)
                            .on_click(cx.listener(|this, checked: &bool, window, cx| {
                                this.indent_guides = *checked;
                                this.editor.update(cx, |state, cx| {
                                    state.set_indent_guides(this.indent_guides, window, cx);
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Switch::new("soft-wrap")
                            .label("Soft Wrap")
                            .checked(self.soft_wrap)
                            .on_click(cx.listener(|this, checked: &bool, window, cx| {
                                this.soft_wrap = *checked;
                                this.editor.update(cx, |state, cx| {
                                    state.set_soft_wrap(this.soft_wrap, window, cx);
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Switch::new("show-whitespaces")
                            .label("Show Whitespaces")
                            .checked(self.show_whitespaces)
                            .on_click(cx.listener(|this, checked: &bool, window, cx| {
                                this.show_whitespaces = *checked;
                                this.editor.update(cx, |state, cx| {
                                    state.set_show_whitespaces(this.show_whitespaces, window, cx);
                                });
                                cx.notify();
                            })),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(
                        Input::new(&self.editor)
                            .bordered(false)
                            .p_0()
                            .h_full()
                            .font_family(cx.theme().mono_font_family.clone())
                            .text_size(cx.theme().mono_font_size)
                            .focus_bordered(false)
                            .into_any_element(),
                    ),
            )
            .child(
                h_flex()
                    .justify_between()
                    .text_sm()
                    .bg(cx.theme().background)
                    .py_1p5()
                    .px_4()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .text_color(cx.theme().muted_foreground)
                    .child(h_flex().gap_2().child(IconName::File).child("example.rs"))
                    .child(Button::new("cursor-position").ghost().xsmall().label(format!(
                        "Ln {}, Col {} ({} bytes)",
                        position.line + 1,
                        position.character + 1,
                        cursor
                    ))),
            )
    }
}
