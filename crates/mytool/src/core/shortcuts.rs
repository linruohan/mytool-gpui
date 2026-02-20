/// 键盘快捷键系统
///
/// 提供统一的快捷键管理和注册功能
///
/// 快捷键分类：
/// - 任务操作：添加、编辑、删除、完成任务
/// - 导航：在不同视图间切换
/// - 搜索和过滤：快速查找任务
/// - 窗口管理：关闭、最小化等
use gpui::{Action, actions};

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
    /// 显示所有任务 (Cmd/Ctrl + Shift + A)
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
    pub fn name(&self) -> &'static str {
        match self {
            Self::Task => "任务操作",
            Self::Navigation => "导航",
            Self::Search => "搜索和过滤",
            Self::Selection => "选择和批量操作",
            Self::Project => "项目和分区",
            Self::View => "视图和窗口",
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
        // 项目和分区
        ShortcutConfig {
            action: "NewProject",
            key: "cmd-shift-n",
            description: "新建项目",
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
            key: "cmd-shift-d",
            description: "删除项目",
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

/// 按分类获取快捷键
pub fn get_shortcuts_by_category(category: ShortcutCategory) -> Vec<ShortcutConfig> {
    get_all_shortcuts().into_iter().filter(|s| s.category == category).collect()
}

/// 生成快捷键帮助文档
pub fn generate_shortcuts_help() -> String {
    let mut help = String::from("# 键盘快捷键\n\n");

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
                shortcut.description,
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
        assert!(help.contains("# 键盘快捷键"));
        assert!(help.contains("## 任务操作"));
        assert!(help.contains("## 导航"));
    }
}
