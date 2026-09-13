/// 键盘快捷键系统
///
/// 提供统一的快捷键管理和注册功能
///
/// 快捷键分类：
/// - 任务操作：添加、编辑、删除、完成任务
/// - 导航：在不同视图间切换
/// - 搜索和过滤：快速查找任务
/// - 窗口管理：关闭、最小化等
use gpui::{App, KeyBinding, actions};
use rust_i18n::t;

// ==================== 任务操作快捷键 ====================

actions!(task_shortcuts, [
    /// 新建任务 (Cmd/Ctrl + N)
    NewTask,
    /// 编辑任务 (Cmd/Ctrl + E)
    EditTask,
    /// 删除任务 (Cmd/Ctrl + D 或 Delete)
    DeleteTask,
    /// 完成/取消完成任务 (Cmd/Ctrl + Enter)
    ToggleTaskComplete,
    /// 置顶/取消置顶任务 (Cmd/Ctrl + P)
    ToggleTaskPin,
    /// 复制任务 (Cmd/Ctrl + Shift + D)
    DuplicateTask,
    /// 移动任务到项目 (Cmd/Ctrl + M)
    MoveTaskToProject,
    /// 设置任务优先级 (Cmd/Ctrl + 1/2/3)
    SetTaskPriority,
    /// 添加标签 (Cmd/Ctrl + L)
    AddLabel,
    /// 设置截止日期 (Cmd/Ctrl + T)
    SetDueDate,
    /// 撤销最近一次任务操作 (Cmd/Ctrl + Z)
    UndoLastTask,
    /// 重做 (Cmd/Ctrl + Shift + Z)
    RedoLastTask,
]);

// ==================== 导航快捷键 ====================

actions!(navigation_shortcuts, [
    /// 显示收件箱 (Cmd/Ctrl + 1)
    ShowInbox,
    /// 显示今日任务 (Cmd/Ctrl + 2)
    ShowToday,
    /// 显示计划任务 (Cmd/Ctrl + 3)
    ShowScheduled,
    /// 显示已完成任务 (Cmd/Ctrl + 4)
    ShowCompleted,
    /// 显示置顶任务 (Cmd/Ctrl + 5)
    ShowPinned,
    /// 显示标签视图 (Cmd/Ctrl + 6)
    ShowLabels,
    /// 下一个视图 (Cmd/Ctrl + ])
    NextView,
    /// 上一个视图 (Cmd/Ctrl + [)
    PreviousView,
    /// 返回 (Cmd/Ctrl + Left)
    GoBack,
    /// 前进 (Cmd/Ctrl + Right)
    GoForward,
]);

// ==================== 搜索和过滤快捷键 ====================

actions!(search_shortcuts, [
    /// 搜索任务 (Cmd/Ctrl + F)
    SearchTasks,
    /// 按标签过滤 (Cmd/Ctrl + Shift + L)
    FilterByLabel,
    /// 按项目过滤 (Cmd/Ctrl + Shift + P)
    FilterByProject,
    /// 按优先级过滤 (Cmd/Ctrl + Shift + 1/2/3)
    FilterByPriority,
    /// 清除过滤器 (Cmd/Ctrl + Shift + C)
    ClearFilters,
    /// 显示所有任务 / 关闭过滤 (Cmd/Ctrl + Shift + H)
    ShowAllTasks,
]);

// ==================== 选择和批量操作快捷键 ====================

actions!(selection_shortcuts, [
    /// 选择所有任务 (Cmd/Ctrl + A)
    SelectAllTasks,
    /// 取消选择 (Esc)
    DeselectAll,
    /// 选择上一个任务 (Up)
    SelectPreviousTask,
    /// 选择下一个任务 (Down)
    SelectNextTask,
    /// 批量完成选中任务 (Cmd/Ctrl + Shift + Enter)
    BatchCompleteSelected,
    /// 批量删除选中任务 (Cmd/Ctrl + Shift + Delete)
    BatchDeleteSelected,
    /// 批量移动选中任务 (Cmd/Ctrl + Shift + M)
    BatchMoveSelected,
    /// 当前任务在分组内上移 (Alt + Up)
    MoveTaskUp,
    /// 当前任务在分组内下移 (Alt + Down)
    MoveTaskDown,
]);

// ==================== 项目和分区快捷键 ====================

actions!(project_shortcuts, [
    /// 新建项目 (Cmd/Ctrl + Shift + N)
    NewProject,
    /// 编辑项目 (Cmd/Ctrl + Shift + E)
    EditProject,
    /// 删除项目 (Cmd/Ctrl + Shift + D)
    DeleteProject,
    /// 归档项目 (Cmd/Ctrl + Shift + A)
    ArchiveProject,
    /// 收藏/取消收藏当前项目 (Cmd/Ctrl + Shift + S)
    ToggleProjectFavorite,
    /// 收藏/取消收藏当前标签 (Cmd/Ctrl + Alt + F)
    ToggleLabelFavorite,
    /// 新建分区 (Cmd/Ctrl + Alt + N)
    NewSection,
    /// 编辑分区 (Cmd/Ctrl + Alt + E)
    EditSection,
    /// 删除分区 (Cmd/Ctrl + Alt + D)
    DeleteSection,
]);

// ==================== 视图和窗口快捷键 ====================

actions!(view_shortcuts, [
    /// 切换侧边栏 (Cmd/Ctrl + B)
    ToggleSidebar,
    /// 切换全屏 (Cmd/Ctrl + Shift + F)
    ToggleFullscreen,
    /// 放大 (Cmd/Ctrl + =)
    ZoomIn,
    /// 缩小 (Cmd/Ctrl + -)
    ZoomOut,
    /// 重置缩放 (Cmd/Ctrl + 0)
    ResetZoom,
    /// 刷新视图 (Cmd/Ctrl + R)
    RefreshView,
    /// 打开设置 (Cmd/Ctrl + ,)
    OpenSettings,
    /// 打开帮助 (F1)
    OpenHelp,
]);

// ==================== 快捷键配置 ====================

/// 快捷键配置
#[derive(Debug, Clone)]
pub struct ShortcutConfig {
    pub action: &'static str,
    pub key: &'static str,
    pub description: &'static str,
    pub category: ShortcutCategory,
}

/// 快捷键分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutCategory {
    Task,
    Navigation,
    Search,
    Selection,
    Project,
    View,
}

impl ShortcutCategory {
    pub fn name(&self) -> String {
        match self {
            Self::Task => t!("todo.shortcut.cat.task").to_string(),
            Self::Navigation => t!("todo.shortcut.cat.navigation").to_string(),
            Self::Search => t!("todo.shortcut.cat.search").to_string(),
            Self::Selection => t!("todo.shortcut.cat.selection").to_string(),
            Self::Project => t!("todo.shortcut.cat.project").to_string(),
            Self::View => t!("todo.shortcut.cat.view").to_string(),
        }
    }
}

impl ShortcutConfig {
    pub fn localized_description(&self) -> String {
        match self.action {
            "NewTask" => t!("todo.item.new").to_string(),
            "EditTask" => t!("todo.item.edit").to_string(),
            "DeleteTask" => t!("todo.item.delete").to_string(),
            "ToggleTaskComplete" => t!("todo.shortcut.toggle_complete").to_string(),
            "ToggleTaskPin" => t!("todo.shortcut.toggle_pin").to_string(),
            "DuplicateTask" => t!("todo.shortcut.duplicate").to_string(),
            "AddLabel" => t!("todo.shortcut.add_label").to_string(),
            "SetDueDate" => t!("todo.due.set_title").to_string(),
            "SetTaskPriority" => t!("todo.shortcut.cycle_priority").to_string(),
            "MoveTaskToProject" => t!("todo.project.move_title").to_string(),
            "UndoLastTask" => t!("todo.shortcut.undo").to_string(),
            "RedoLastTask" => t!("todo.shortcut.redo").to_string(),
            "ShowInbox" => t!("todo.shortcut.show_inbox").to_string(),
            "ShowToday" => t!("todo.shortcut.show_today").to_string(),
            "ShowScheduled" => t!("todo.shortcut.show_scheduled").to_string(),
            "ShowCompleted" => t!("todo.shortcut.show_completed").to_string(),
            "ShowPinned" => t!("todo.shortcut.show_pinned").to_string(),
            "ShowLabels" => t!("todo.shortcut.show_labels").to_string(),
            "NextView" => t!("todo.shortcut.next_view").to_string(),
            "PreviousView" => t!("todo.shortcut.prev_view").to_string(),
            "GoBack" => t!("todo.shortcut.go_back").to_string(),
            "GoForward" => t!("todo.shortcut.go_forward").to_string(),
            "SearchTasks" => t!("todo.shortcut.search").to_string(),
            "FilterByLabel" => t!("todo.filter.labels").to_string(),
            "FilterByProject" => t!("todo.filter.projects").to_string(),
            "FilterByPriority" => t!("todo.filter.priority").to_string(),
            "ClearFilters" => t!("todo.shortcut.clear_filters").to_string(),
            "ShowAllTasks" => t!("todo.shortcut.show_all").to_string(),
            "SelectAllTasks" => t!("todo.shortcut.select_all").to_string(),
            "DeselectAll" => t!("todo.shortcut.deselect").to_string(),
            "SelectPreviousTask" => t!("todo.shortcut.select_prev").to_string(),
            "SelectNextTask" => t!("todo.shortcut.select_next").to_string(),
            "BatchCompleteSelected" => t!("todo.batch.complete").to_string(),
            "BatchDeleteSelected" => t!("todo.batch.delete").to_string(),
            "BatchMoveSelected" => t!("todo.batch.move").to_string(),
            "MoveTaskUp" => t!("todo.shortcut.move_up").to_string(),
            "MoveTaskDown" => t!("todo.shortcut.move_down").to_string(),
            "NewProject" => t!("todo.project.new").to_string(),
            "EditProject" => t!("todo.shortcut.edit_project").to_string(),
            "DeleteProject" => t!("todo.shortcut.delete_project").to_string(),
            "ArchiveProject" => t!("todo.project.archive").to_string(),
            "ToggleProjectFavorite" => t!("todo.project.favorite_toggle").to_string(),
            "ToggleLabelFavorite" => t!("todo.label.favorite_toggle").to_string(),
            "NewSection" => t!("todo.section.new").to_string(),
            "EditSection" => t!("todo.section.edit").to_string(),
            "DeleteSection" => t!("todo.section.delete").to_string(),
            "ToggleSidebar" => t!("todo.shortcut.toggle_sidebar").to_string(),
            "ToggleFullscreen" => t!("todo.shortcut.toggle_fullscreen").to_string(),
            "ZoomIn" => t!("todo.shortcut.zoom_in").to_string(),
            "ZoomOut" => t!("todo.shortcut.zoom_out").to_string(),
            "ResetZoom" => t!("todo.shortcut.reset_zoom").to_string(),
            "RefreshView" => t!("todo.shortcut.refresh").to_string(),
            "OpenSettings" => t!("todo.shortcut.open_settings").to_string(),
            "OpenHelp" => t!("todo.shortcut.open_help").to_string(),
            _ => self.description.to_string(),
        }
    }
}

/// 获取所有快捷键配置
pub fn get_all_shortcuts() -> Vec<ShortcutConfig> {
    vec![
        // 任务操作
        ShortcutConfig {
            action: "NewTask",
            key: "cmd-n",
            description: "新建任务",
            category: ShortcutCategory::Task,
        },
        ShortcutConfig {
            action: "EditTask",
            key: "cmd-e",
            description: "编辑任务",
            category: ShortcutCategory::Task,
        },
        ShortcutConfig {
            action: "DeleteTask",
            key: "cmd-d",
            description: "删除任务",
            category: ShortcutCategory::Task,
        },
        ShortcutConfig {
            action: "ToggleTaskComplete",
            key: "cmd-enter",
            description: "完成/取消完成任务",
            category: ShortcutCategory::Task,
        },
        ShortcutConfig {
            action: "ToggleTaskPin",
            key: "cmd-p",
            description: "置顶/取消置顶任务",
            category: ShortcutCategory::Task,
        },
        ShortcutConfig {
            action: "DuplicateTask",
            key: "cmd-shift-d",
            description: "复制任务",
            category: ShortcutCategory::Task,
        },
        ShortcutConfig {
            action: "AddLabel",
            key: "cmd-l",
            description: "添加标签",
            category: ShortcutCategory::Task,
        },
        ShortcutConfig {
            action: "SetDueDate",
            key: "cmd-t",
            description: "设置截止日期",
            category: ShortcutCategory::Task,
        },
        ShortcutConfig {
            action: "SetTaskPriority",
            key: "alt-p",
            description: "循环任务优先级",
            category: ShortcutCategory::Task,
        },
        ShortcutConfig {
            action: "MoveTaskToProject",
            key: "cmd-m",
            description: "移动任务到项目",
            category: ShortcutCategory::Task,
        },
        ShortcutConfig {
            action: "UndoLastTask",
            key: "cmd-z",
            description: "撤销任务操作",
            category: ShortcutCategory::Task,
        },
        ShortcutConfig {
            action: "RedoLastTask",
            key: "cmd-shift-z",
            description: "重做任务操作",
            category: ShortcutCategory::Task,
        },
        // 导航
        ShortcutConfig {
            action: "ShowInbox",
            key: "cmd-1",
            description: "显示收件箱",
            category: ShortcutCategory::Navigation,
        },
        ShortcutConfig {
            action: "ShowToday",
            key: "cmd-2",
            description: "显示今日任务",
            category: ShortcutCategory::Navigation,
        },
        ShortcutConfig {
            action: "ShowScheduled",
            key: "cmd-3",
            description: "显示计划任务",
            category: ShortcutCategory::Navigation,
        },
        ShortcutConfig {
            action: "ShowCompleted",
            key: "cmd-4",
            description: "显示已完成任务",
            category: ShortcutCategory::Navigation,
        },
        ShortcutConfig {
            action: "ShowPinned",
            key: "cmd-5",
            description: "显示置顶任务",
            category: ShortcutCategory::Navigation,
        },
        ShortcutConfig {
            action: "ShowLabels",
            key: "cmd-6",
            description: "显示标签视图",
            category: ShortcutCategory::Navigation,
        },
        ShortcutConfig {
            action: "NextView",
            key: "cmd-]",
            description: "下一个看板",
            category: ShortcutCategory::Navigation,
        },
        ShortcutConfig {
            action: "PreviousView",
            key: "cmd-[",
            description: "上一个看板",
            category: ShortcutCategory::Navigation,
        },
        ShortcutConfig {
            action: "GoBack",
            key: "cmd-left",
            description: "后退",
            category: ShortcutCategory::Navigation,
        },
        ShortcutConfig {
            action: "GoForward",
            key: "cmd-right",
            description: "前进",
            category: ShortcutCategory::Navigation,
        },
        // 搜索和过滤
        ShortcutConfig {
            action: "SearchTasks",
            key: "cmd-f",
            description: "搜索任务",
            category: ShortcutCategory::Search,
        },
        ShortcutConfig {
            action: "FilterByLabel",
            key: "cmd-shift-l",
            description: "按标签过滤",
            category: ShortcutCategory::Search,
        },
        ShortcutConfig {
            action: "FilterByProject",
            key: "cmd-shift-p",
            description: "按项目过滤",
            category: ShortcutCategory::Search,
        },
        ShortcutConfig {
            action: "ClearFilters",
            key: "cmd-shift-c",
            description: "清除过滤器",
            category: ShortcutCategory::Search,
        },
        ShortcutConfig {
            action: "ShowAllTasks",
            key: "cmd-shift-h",
            description: "显示全部并关闭过滤",
            category: ShortcutCategory::Search,
        },
        ShortcutConfig {
            action: "FilterByPriority",
            key: "cmd-shift-1",
            description: "按优先级过滤",
            category: ShortcutCategory::Search,
        },
        // 选择和批量操作
        ShortcutConfig {
            action: "SelectAllTasks",
            key: "cmd-a",
            description: "选择所有任务",
            category: ShortcutCategory::Selection,
        },
        ShortcutConfig {
            action: "DeselectAll",
            key: "escape",
            description: "取消选择",
            category: ShortcutCategory::Selection,
        },
        ShortcutConfig {
            action: "SelectPreviousTask",
            key: "up",
            description: "选择上一个任务",
            category: ShortcutCategory::Selection,
        },
        ShortcutConfig {
            action: "SelectNextTask",
            key: "down",
            description: "选择下一个任务",
            category: ShortcutCategory::Selection,
        },
        ShortcutConfig {
            action: "BatchCompleteSelected",
            key: "cmd-shift-enter",
            description: "批量完成选中任务",
            category: ShortcutCategory::Selection,
        },
        ShortcutConfig {
            action: "BatchDeleteSelected",
            key: "cmd-shift-delete",
            description: "批量删除选中任务",
            category: ShortcutCategory::Selection,
        },
        ShortcutConfig {
            action: "BatchMoveSelected",
            key: "cmd-shift-m",
            description: "批量移动选中任务",
            category: ShortcutCategory::Selection,
        },
        ShortcutConfig {
            action: "MoveTaskUp",
            key: "alt-up",
            description: "当前任务上移",
            category: ShortcutCategory::Selection,
        },
        ShortcutConfig {
            action: "MoveTaskDown",
            key: "alt-down",
            description: "当前任务下移",
            category: ShortcutCategory::Selection,
        },
        // 项目和分区
        ShortcutConfig {
            action: "NewProject",
            key: "cmd-shift-n",
            description: "新建项目",
            category: ShortcutCategory::Project,
        },
        ShortcutConfig {
            action: "NewSection",
            key: "cmd-alt-n",
            description: "新建分区",
            category: ShortcutCategory::Project,
        },
        ShortcutConfig {
            action: "EditSection",
            key: "cmd-alt-e",
            description: "编辑分区",
            category: ShortcutCategory::Project,
        },
        ShortcutConfig {
            action: "DeleteSection",
            key: "cmd-alt-d",
            description: "删除分区",
            category: ShortcutCategory::Project,
        },
        ShortcutConfig {
            action: "EditProject",
            key: "cmd-shift-e",
            description: "编辑项目",
            category: ShortcutCategory::Project,
        },
        ShortcutConfig {
            action: "DeleteProject",
            key: "cmd-shift-delete",
            description: "删除项目",
            category: ShortcutCategory::Project,
        },
        ShortcutConfig {
            action: "ArchiveProject",
            key: "cmd-shift-a",
            description: "归档项目",
            category: ShortcutCategory::Project,
        },
        ShortcutConfig {
            action: "ToggleProjectFavorite",
            key: "cmd-shift-s",
            description: "收藏当前项目",
            category: ShortcutCategory::Project,
        },
        ShortcutConfig {
            action: "ToggleLabelFavorite",
            key: "cmd-alt-f",
            description: "收藏当前标签",
            category: ShortcutCategory::Project,
        },
        // 视图和窗口
        ShortcutConfig {
            action: "ToggleSidebar",
            key: "cmd-b",
            description: "切换侧边栏",
            category: ShortcutCategory::View,
        },
        ShortcutConfig {
            action: "ToggleFullscreen",
            key: "cmd-shift-f",
            description: "切换全屏",
            category: ShortcutCategory::View,
        },
        ShortcutConfig {
            action: "ZoomIn",
            key: "cmd-=",
            description: "放大",
            category: ShortcutCategory::View,
        },
        ShortcutConfig {
            action: "ZoomOut",
            key: "cmd--",
            description: "缩小",
            category: ShortcutCategory::View,
        },
        ShortcutConfig {
            action: "ResetZoom",
            key: "cmd-0",
            description: "重置缩放",
            category: ShortcutCategory::View,
        },
        ShortcutConfig {
            action: "RefreshView",
            key: "cmd-r",
            description: "刷新视图",
            category: ShortcutCategory::View,
        },
        ShortcutConfig {
            action: "OpenSettings",
            key: "cmd-,",
            description: "打开设置",
            category: ShortcutCategory::View,
        },
        ShortcutConfig {
            action: "OpenHelp",
            key: "f1",
            description: "打开帮助",
            category: ShortcutCategory::View,
        },
    ]
}

/// 将 Todo 视图快捷键绑到 `TodoStory` 键上下文（Gallery 入口保持不变）
pub fn bind_todo_keys(cx: &mut App) {
    const CTX: Option<&str> = Some("TodoStory");
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-n", NewTask, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-n", NewTask, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-f", SearchTasks, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-f", SearchTasks, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-1", ShowInbox, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-1", ShowInbox, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-2", ShowToday, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-2", ShowToday, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-3", ShowScheduled, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-3", ShowScheduled, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-4", ShowLabels, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-4", ShowLabels, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-5", ShowPinned, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-5", ShowPinned, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-6", ShowCompleted, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-6", ShowCompleted, CTX),
        KeyBinding::new("escape", DeselectAll, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", SelectAllTasks, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", SelectAllTasks, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-enter", BatchCompleteSelected, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-enter", BatchCompleteSelected, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-backspace", BatchDeleteSelected, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-backspace", BatchDeleteSelected, CTX),
        KeyBinding::new("alt-up", MoveTaskUp, CTX),
        KeyBinding::new("alt-down", MoveTaskDown, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-,", OpenSettings, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-,", OpenSettings, CTX),
        KeyBinding::new("f1", OpenHelp, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-z", UndoLastTask, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-z", UndoLastTask, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-z", RedoLastTask, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-z", RedoLastTask, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-y", RedoLastTask, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-e", EditTask, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-e", EditTask, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-d", DeleteTask, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-d", DeleteTask, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-enter", ToggleTaskComplete, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-enter", ToggleTaskComplete, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-d", DuplicateTask, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-d", DuplicateTask, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-p", ToggleTaskPin, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-p", ToggleTaskPin, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-t", SetDueDate, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-t", SetDueDate, CTX),
        KeyBinding::new("up", SelectPreviousTask, CTX),
        KeyBinding::new("down", SelectNextTask, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-b", ToggleSidebar, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-b", ToggleSidebar, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-n", NewProject, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-n", NewProject, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-alt-n", NewSection, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-alt-n", NewSection, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-alt-e", EditSection, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-alt-e", EditSection, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-alt-d", DeleteSection, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-alt-d", DeleteSection, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-e", EditProject, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-e", EditProject, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-delete", DeleteProject, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-delete", DeleteProject, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-a", ArchiveProject, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-a", ArchiveProject, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-s", ToggleProjectFavorite, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-s", ToggleProjectFavorite, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-alt-f", ToggleLabelFavorite, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-alt-f", ToggleLabelFavorite, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-l", AddLabel, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-l", AddLabel, CTX),
        KeyBinding::new("alt-p", SetTaskPriority, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-m", MoveTaskToProject, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-m", MoveTaskToProject, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-m", BatchMoveSelected, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-m", BatchMoveSelected, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-]", NextView, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-]", NextView, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-[", PreviousView, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-[", PreviousView, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-left", GoBack, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-left", GoBack, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-right", GoForward, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-right", GoForward, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-l", FilterByLabel, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-l", FilterByLabel, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-p", FilterByProject, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-p", FilterByProject, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-1", FilterByPriority, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-1", FilterByPriority, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-2", FilterByPriority, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-2", FilterByPriority, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-3", FilterByPriority, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-3", FilterByPriority, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-c", ClearFilters, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-c", ClearFilters, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-h", ShowAllTasks, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-h", ShowAllTasks, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-r", RefreshView, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-r", RefreshView, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-=", ZoomIn, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-=", ZoomIn, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-+", ZoomIn, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-+", ZoomIn, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd--", ZoomOut, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl--", ZoomOut, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-0", ResetZoom, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-0", ResetZoom, CTX),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-f", ToggleFullscreen, CTX),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-f", ToggleFullscreen, CTX),
        KeyBinding::new("f11", ToggleFullscreen, CTX),
    ]);
}

/// 按分类获取快捷键
pub fn get_shortcuts_by_category(category: ShortcutCategory) -> Vec<ShortcutConfig> {
    get_all_shortcuts().into_iter().filter(|s| s.category == category).collect()
}

/// 生成快捷键帮助文档
pub fn generate_shortcuts_help() -> String {
    let mut help = format!("# {}\n\n", t!("todo.shortcut.help_title"));

    for category in [
        ShortcutCategory::Task,
        ShortcutCategory::Navigation,
        ShortcutCategory::Search,
        ShortcutCategory::Selection,
        ShortcutCategory::Project,
        ShortcutCategory::View,
    ] {
        help.push_str(&format!("## {}\n\n", category.name()));

        for shortcut in get_shortcuts_by_category(category) {
            help.push_str(&format!(
                "- **{}**: {} ({})\n",
                shortcut.localized_description(),
                shortcut.key.replace("cmd", "Cmd/Ctrl"),
                shortcut.action
            ));
        }

        help.push('\n');
    }

    help
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_shortcuts() {
        let shortcuts = get_all_shortcuts();
        assert!(!shortcuts.is_empty());
        assert!(shortcuts.len() > 30); // 至少有 30 个快捷键
    }

    #[test]
    fn test_get_shortcuts_by_category() {
        let task_shortcuts = get_shortcuts_by_category(ShortcutCategory::Task);
        assert!(!task_shortcuts.is_empty());

        for shortcut in task_shortcuts {
            assert_eq!(shortcut.category, ShortcutCategory::Task);
        }
    }

    #[test]
    fn test_generate_shortcuts_help() {
        let help = generate_shortcuts_help();
        assert!(help.starts_with('#') && help.contains("NewTask"));
        assert!(help.contains("## "));
    }
}
