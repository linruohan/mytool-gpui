use gpui::{
    AnyView, App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, Styled, Subscription, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    Sizable, WindowExt,
    button::Button,
    notification::{Notification, NotificationType},
    v_flex,
};
use gpui_fps::fps_monitor;
use rust_i18n::t;

use crate::{
    AppState, AppTitleBar, ShowPanelInfo, ToggleSearch,
    core::state::{ErrorNotifier, ReminderNotifier, TodoPrefs},
};

pub struct StoryRoot {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) title_bar: Entity<AppTitleBar>,
    pub(crate) view: AnyView,
    _subscriptions: Vec<Subscription>,
}

impl StoryRoot {
    pub fn new(
        title: impl Into<SharedString>,
        view: impl Into<AnyView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new(title, window, cx));
        let error_sub = cx.observe_global_in::<ErrorNotifier>(window, |_, window, cx| {
            present_pending_errors(window, cx);
        });
        let reminder_sub = cx.observe_global_in::<ReminderNotifier>(window, |_, window, cx| {
            present_pending_reminders(window, cx);
        });
        // 观察者不会回放订阅前已经写入的消息。
        present_pending_errors(window, cx);
        present_pending_reminders(window, cx);
        Self {
            focus_handle: cx.focus_handle(),
            title_bar,
            view: view.into(),
            _subscriptions: vec![error_sub, reminder_sub],
        }
    }

    fn on_action_panel_info(
        &mut self,
        _: &ShowPanelInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        struct Info;
        let note = Notification::new().message("You have clicked panel info.").id::<Info>();
        window.push_notification(note, cx);
    }

    fn on_action_toggle_search(
        &mut self,
        _: &ToggleSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.propagate();
        if window.has_focused_input(cx) {
            return;
        }

        struct Search;
        let note = Notification::new().message("You have toggled search.").id::<Search>();
        window.push_notification(note, cx);
    }
}

/// 到期提醒弹层的标识，延后时按提醒 id 关掉对应的那一条。
struct DueReminder;

struct ShownError;

fn present_pending_errors(window: &mut Window, cx: &mut App) {
    for msg in cx.global::<ErrorNotifier>().take_all() {
        window.push_notification(
            Notification::error(msg.clone()).id1::<ShownError>(SharedString::from(msg)),
            cx,
        );
    }
}

const REMINDER_LINES: usize = 5;

fn reminder_batch_message(titles: &[String]) -> String {
    let mut lines: Vec<String> = titles.iter().take(REMINDER_LINES).cloned().collect();
    if titles.len() > REMINDER_LINES {
        let extra = titles.len() - REMINDER_LINES;
        lines.push(t!("todo.reminder.due_more", count => extra).to_string());
    }
    lines.join("\n")
}

fn push_reminder_toast(
    window: &mut Window,
    cx: &mut App,
    window_active: bool,
    reminders: Vec<(String, String)>,
    message: String,
    open_item_id: Option<String>,
) {
    let note_key = SharedString::from(
        reminders.iter().map(|(reminder_id, _)| reminder_id.as_str()).collect::<Vec<_>>().join(","),
    );
    let button_id = SharedString::from(format!(
        "snooze-{}",
        reminders.first().map(|(reminder_id, _)| reminder_id.as_str()).unwrap_or("batch")
    ));
    let mut note = Notification::new()
        .id1::<DueReminder>(note_key.clone())
        .title(t!("todo.reminder.title").to_string())
        .message(message)
        .with_type(NotificationType::Info)
        .on_close({
            let reminders = reminders.clone();
            move |_window, cx| {
                for (reminder_id, item_id) in &reminders {
                    if cx.global::<ReminderNotifier>().take_snooze(reminder_id) {
                        continue;
                    }
                    crate::todo_actions::acknowledge_reminder(
                        reminder_id.clone(),
                        item_id.clone(),
                        cx,
                    );
                }
            }
        })
        .action({
            let reminders = reminders.clone();
            let note_key = note_key.clone();
            move |_, _, _| {
                let reminders = reminders.clone();
                let note_key = note_key.clone();
                Button::new(button_id.clone())
                    .small()
                    .label(t!("todo.reminder.snooze").to_string())
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        for (reminder_id, item_id) in &reminders {
                            cx.global::<ReminderNotifier>().begin_snooze(reminder_id);
                            crate::todo_actions::snooze_reminder(
                                reminder_id.clone(),
                                item_id.clone(),
                                15,
                                cx,
                            );
                        }
                        window.remove_notification1::<DueReminder>(note_key.clone(), cx);
                        window.push_notification(t!("todo.reminder.snoozed").to_string(), cx);
                    })
            }
        })
        .when_some(open_item_id, |note, item_id| {
            note.on_click(move |_event, window, cx| {
                crate::ui::components::open_reminder_item(window, cx, &item_id);
            })
        });
    if !window_active {
        note = note.in_app_and_system();
    }
    window.push_notification(note, cx);
}

fn present_pending_reminders(window: &mut Window, cx: &mut App) {
    let notices = cx.global::<ReminderNotifier>().take_all();
    if notices.is_empty() {
        return;
    }
    let window_active = window.is_window_active();
    for notice in &notices {
        cx.global::<ReminderNotifier>().hold(&notice.reminder_id);
    }
    let reminders: Vec<(String, String)> =
        notices.iter().map(|notice| (notice.reminder_id.clone(), notice.item_id.clone())).collect();
    let message = if notices.len() == 1 {
        notices[0].title.clone()
    } else {
        reminder_batch_message(
            &notices.iter().map(|notice| notice.title.clone()).collect::<Vec<_>>(),
        )
    };
    let open_item_id = if notices.len() == 1 && !notices[0].item_id.is_empty() {
        Some(notices[0].item_id.clone())
    } else {
        None
    };
    push_reminder_toast(window, cx, window_active, reminders, message, open_item_id);
    if window_active {
        let _ = crate::play_success_sound();
    }
}

impl Focusable for StoryRoot {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StoryRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let scale = cx.global::<TodoPrefs>().rem_px();
        window.set_rem_size(px(scale));
        // 弹层（sheet/dialog/notification）由外层 Root 的 WindowState 插件自动渲染，
        // 新版 gpui-component 不再需要（也不支持）手动渲染图层
        let show_fps = AppState::global(cx).show_fps_monitor;

        div()
            .id("story-root")
            .on_action(cx.listener(Self::on_action_panel_info))
            .on_action(cx.listener(Self::on_action_toggle_search))
            .size_full()
            .child(
                v_flex().size_full().child(self.title_bar.clone()).child(
                    div()
                        .track_focus(&self.focus_handle)
                        .flex_1()
                        .overflow_hidden()
                        .child(self.view.clone())
                        // Rendered inside the content area rather than the
                        // window root so the HUD's top right corner clears
                        // the title bar's own controls.
                        .when(show_fps, |this| this.child(fps_monitor(window, cx))),
                ),
            )
    }
}
