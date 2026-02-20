# Visual Optimization Guide

## 概述

本指南介绍 MyTool 应用的视觉优化系统，包括语义化颜色、视觉层次、动画效果和响应式布局。

## 目录

1. [语义化颜色系统](#语义化颜色系统)
2. [视觉层次](#视觉层次)
3. [动画和过渡](#动画和过渡)
4. [响应式布局](#响应式布局)
5. [使用示例](#使用示例)
6. [最佳实践](#最佳实践)

---

## 语义化颜色系统

### 设计理念

语义化颜色为不同的 UI 元素提供一致的视觉含义，提升用户体验和可访问性。

### 颜色分类

#### 1. 优先级颜色

用于标识任务的优先级：

- **高优先级** (Priority High): 红色系
  - 用途：紧急任务、重要提醒
  - 视觉效果：醒目、引起注意
  
- **中优先级** (Priority Medium): 黄色系
  - 用途：重要但不紧急的任务
  - 视觉效果：温和提醒
  
- **低优先级** (Priority Low): 蓝色系
  - 用途：常规任务
  - 视觉效果：平和、不突兀
  
- **无优先级** (Priority None): 灰色系
  - 用途：未设置优先级的任务
  - 视觉效果：中性

#### 2. 状态颜色

用于标识任务的状态：

- **已完成** (Completed): 绿色
  - 用途：已完成的任务
  - 视觉效果：积极、成功
  
- **逾期** (Overdue): 红色
  - 用途：超过截止日期的任务
  - 视觉效果：警告、需要关注
  
- **今日** (Today): 橙色
  - 用途：今天需要完成的任务
  - 视觉效果：提醒、行动
  
- **已计划** (Scheduled): 蓝色
  - 用途：已安排时间的任务
  - 视觉效果：有序、计划中
  
- **置顶** (Pinned): 紫色
  - 用途：置顶的重要任务
  - 视觉效果：特殊、突出

#### 3. 交互颜色

用于用户交互反馈：

- **悬停覆盖** (Hover Overlay): 半透明白色/黑色
  - 用途：鼠标悬停时的视觉反馈
  - 视觉效果：微妙的高亮
  
- **激活覆盖** (Active Overlay): 半透明蓝色
  - 用途：点击或选中时的视觉反馈
  - 视觉效果：明确的选中状态
  
- **焦点环** (Focus Ring): 蓝色
  - 用途：键盘导航时的焦点指示
  - 视觉效果：清晰的焦点边框
  
- **拖拽覆盖** (Drag Overlay): 半透明蓝色
  - 用途：拖拽操作时的视觉反馈
  - 视觉效果：拖拽目标区域高亮

#### 4. 反馈颜色

用于系统反馈：

- **成功** (Success): 绿色
- **警告** (Warning): 黄色
- **错误** (Error): 红色
- **信息** (Info): 蓝色

### 使用方法

```rust
use crate::visual_enhancements::SemanticColors;

// 获取语义化颜色
let colors = SemanticColors::from_theme(cx);

// 使用优先级颜色
let priority_color = colors.priority_color(3); // 高优先级

// 使用状态颜色
let status_color = colors.status_completed;

// 应用到 UI 元素
div()
    .border_l_4()
    .border_color(priority_color)
    .child(content)
```

---

## 视觉层次

### 设计理念

通过阴影、边框、圆角和间距创建清晰的视觉层次，帮助用户理解界面结构。

### 阴影系统

三级阴影系统，用于表示不同的深度：

- **小阴影** (Shadow SM): 0 1px 2px rgba(0,0,0,0.05)
  - 用途：轻微的分离效果
  - 示例：输入框、按钮
  
- **中阴影** (Shadow MD): 0 4px 6px rgba(0,0,0,0.1)
  - 用途：卡片、面板
  - 示例：任务卡片、侧边栏
  
- **大阴影** (Shadow LG): 0 10px 15px rgba(0,0,0,0.1)
  - 用途：浮动元素、模态框
  - 示例：下拉菜单、对话框

### 圆角系统

四级圆角系统，用于不同大小的元素：

- **小圆角** (Radius SM): 4px
  - 用途：小按钮、标签
  
- **中圆角** (Radius MD): 6px
  - 用途：输入框、普通按钮
  
- **大圆角** (Radius LG): 8px
  - 用途：卡片、面板
  
- **超大圆角** (Radius XL): 12px
  - 用途：大型容器、模态框

### 间距系统

基于 4px 的间距系统，确保一致性：

- **1x**: 4px - 紧密间距
- **2x**: 8px - 小间距
- **3x**: 12px - 中间距
- **4x**: 16px - 标准间距
- **6x**: 24px - 大间距
- **8x**: 32px - 超大间距

### 使用方法

```rust
use crate::visual_enhancements::VisualHierarchy;

// 应用圆角
div()
    .rounded(VisualHierarchy::radius_md())
    .child(content)

// 应用间距
div()
    .p(VisualHierarchy::spacing(4.0))  // 16px padding
    .gap(VisualHierarchy::spacing(2.0))  // 8px gap
    .child(content)
```

---

## 动画和过渡

### 设计理念

平滑的动画和过渡提升用户体验，使界面感觉更加流畅和响应迅速。

### 动画时长

三种标准时长：

- **快速** (Fast): 150ms
  - 用途：小元素的快速反馈
  - 示例：按钮悬停、复选框切换
  
- **正常** (Normal): 200ms
  - 用途：大多数过渡效果
  - 示例：面板展开、列表项动画
  
- **慢速** (Slow): 300ms
  - 用途：复杂的状态变化
  - 示例：页面切换、模态框显示

### 缓动函数

三种标准缓动函数：

- **Ease In Out**: cubic-bezier(0.4, 0.0, 0.2, 1.0)
  - 用途：大多数动画
  - 效果：开始和结束时缓慢，中间快速
  
- **Ease Out**: cubic-bezier(0.0, 0.0, 0.2, 1.0)
  - 用途：元素进入
  - 效果：开始快速，结束缓慢
  
- **Ease In**: cubic-bezier(0.4, 0.0, 1.0, 1.0)
  - 用途：元素退出
  - 效果：开始缓慢，结束快速

### 使用方法

```rust
use crate::visual_enhancements::Animations;
use std::time::Duration;

// 应用过渡效果（概念示例）
div()
    .transition(Duration::from_millis(Animations::DURATION_NORMAL))
    .when(is_hovered, |this| {
        this.bg(hover_color)
    })
    .child(content)
```

---

## 响应式布局

### 设计理念

根据窗口大小调整布局，确保在不同屏幕尺寸下都有良好的用户体验。

### 断点系统

四个标准断点：

- **SM** (Small): 640px
  - 用途：小屏幕设备
  - 布局：单列、紧凑
  
- **MD** (Medium): 768px
  - 用途：平板设备
  - 布局：双列、适中
  
- **LG** (Large): 1024px
  - 用途：桌面设备
  - 布局：多列、宽松
  
- **XL** (Extra Large): 1280px
  - 用途：大屏幕设备
  - 布局：多列、超宽松

### 布局模式

三种布局模式：

- **紧凑模式** (Compact): < 768px
  - 特点：垂直布局、隐藏次要信息
  - 示例：侧边栏折叠、单列任务列表
  
- **正常模式** (Normal): 768px - 1024px
  - 特点：混合布局、显示主要信息
  - 示例：侧边栏显示、双列任务列表
  
- **宽屏模式** (Wide): >= 1024px
  - 特点：水平布局、显示所有信息
  - 示例：侧边栏展开、多列任务列表

### 使用方法

```rust
use crate::visual_enhancements::ResponsiveLayout;

// 检查布局模式
let window_width = window.viewport_size().width;
let is_compact = ResponsiveLayout::is_compact(window_width);

// 应用响应式布局
v_flex()
    .when(is_compact, |this| {
        // 紧凑模式：垂直布局
        this.flex_col()
            .child(sidebar_compact)
            .child(content)
    })
    .when(!is_compact, |this| {
        // 正常模式：水平布局
        this.flex_row()
            .child(sidebar.w(px(250.0)))
            .child(content.flex_1())
    })
```

---

## 使用示例

### 示例 1: 任务卡片

```rust
use crate::visual_enhancements::{SemanticColors, VisualHierarchy};

fn render_task_card(&self, item: &ItemModel, cx: &App) -> impl IntoElement {
    let colors = SemanticColors::from_theme(cx);
    let priority_color = colors.priority_color(item.priority);
    
    div()
        // 卡片样式
        .rounded(VisualHierarchy::radius_lg())
        .p(VisualHierarchy::spacing(4.0))
        .bg(cx.theme().background)
        .border_1()
        .border_color(cx.theme().border)
        
        // 优先级指示器
        .border_l_4()
        .border_color(priority_color)
        
        // 悬停效果
        .hover(|this| {
            this.bg(colors.hover_overlay)
        })
        
        // 内容
        .child(
            v_flex()
                .gap(VisualHierarchy::spacing(2.0))
                .child(item.content.clone())
                .child(render_metadata(item, cx))
        )
}
```

### 示例 2: 状态徽章

```rust
fn render_status_badge(&self, status: &str, cx: &App) -> impl IntoElement {
    let colors = SemanticColors::from_theme(cx);
    let (bg_color, text) = match status {
        "completed" => (colors.status_completed, "已完成"),
        "overdue" => (colors.status_overdue, "逾期"),
        "today" => (colors.status_today, "今日"),
        _ => (colors.status_scheduled, "已计划"),
    };
    
    div()
        .rounded(VisualHierarchy::radius_sm())
        .px(VisualHierarchy::spacing(2.0))
        .py(VisualHierarchy::spacing(1.0))
        .bg(bg_color)
        .text_color(gpui::white())
        .text_xs()
        .child(text)
}
```

### 示例 3: 响应式侧边栏

```rust
fn render_sidebar(&self, window: &mut Window, cx: &App) -> impl IntoElement {
    let window_width = window.viewport_size().width;
    let is_compact = ResponsiveLayout::is_compact(window_width);
    
    div()
        .when(is_compact, |this| {
            // 紧凑模式：可折叠侧边栏
            this.absolute()
                .left_0()
                .top_0()
                .h_full()
                .w(px(60.0))
                .child(render_compact_sidebar(cx))
        })
        .when(!is_compact, |this| {
            // 正常模式：完整侧边栏
            this.w(px(250.0))
                .h_full()
                .child(render_full_sidebar(cx))
        })
}
```

---

## 最佳实践

### 1. 颜色使用

- ✅ **使用语义化颜色**：优先使用 `SemanticColors` 而不是硬编码颜色
- ✅ **保持一致性**：相同的含义使用相同的颜色
- ✅ **考虑可访问性**：确保足够的对比度（WCAG AA 标准）
- ❌ **避免过度使用颜色**：不要让界面过于花哨

### 2. 视觉层次

- ✅ **使用阴影表示深度**：浮动元素使用更大的阴影
- ✅ **保持圆角一致**：同类元素使用相同的圆角
- ✅ **使用间距系统**：所有间距都应该是 4px 的倍数
- ❌ **避免过度装饰**：保持界面简洁

### 3. 动画效果

- ✅ **使用标准时长**：不要自定义动画时长
- ✅ **选择合适的缓动**：大多数情况使用 ease-in-out
- ✅ **保持流畅**：避免卡顿和延迟
- ❌ **避免过度动画**：不要让用户感到眩晕

### 4. 响应式设计

- ✅ **测试不同尺寸**：确保在所有断点下都能正常工作
- ✅ **优先移动端**：从小屏幕开始设计
- ✅ **渐进增强**：大屏幕添加更多功能
- ❌ **避免固定尺寸**：使用相对单位和弹性布局

### 5. 性能考虑

- ✅ **缓存颜色对象**：避免重复创建 `SemanticColors`
- ✅ **使用条件渲染**：根据窗口大小只渲染需要的内容
- ✅ **优化动画**：使用 GPU 加速的属性（transform, opacity）
- ❌ **避免过度重绘**：减少不必要的状态更新

---

## 实施计划

### 阶段 1: 基础设施（已完成）

- [x] 创建 `visual_enhancements.rs` 模块
- [x] 定义语义化颜色系统
- [x] 定义视觉层次工具
- [x] 定义动画和响应式工具
- [x] 编写使用指南

### 阶段 2: 应用到现有组件（进行中）

- [ ] 更新任务卡片组件
- [ ] 更新侧边栏组件
- [ ] 更新按钮和输入框
- [ ] 更新模态框和对话框
- [ ] 更新列表和表格

### 阶段 3: 增强和优化

- [ ] 添加更多动画效果
- [ ] 优化响应式布局
- [ ] 添加主题切换动画
- [ ] 性能测试和优化

### 阶段 4: 文档和测试

- [ ] 完善使用文档
- [ ] 添加视觉回归测试
- [ ] 创建设计系统文档
- [ ] 用户测试和反馈

---

## 技术细节

### 颜色计算

语义化颜色使用 HSL 颜色空间，便于调整亮度和饱和度：

```rust
// 亮色模式：较低的亮度
hsla(0.0, 0.7, 0.4, 1.0)  // 红色，亮度 40%

// 暗色模式：较高的亮度
hsla(0.0, 0.7, 0.5, 1.0)  // 红色，亮度 50%
```

### 主题适配

所有颜色都会根据当前主题（亮色/暗色）自动调整：

```rust
let is_dark = cx.theme().mode.is_dark();
let color = if is_dark {
    hsla(h, s, l_dark, a)
} else {
    hsla(h, s, l_light, a)
};
```

### 性能优化

- 颜色对象可以缓存，避免重复计算
- 响应式检查可以在窗口大小变化时触发
- 动画使用 GPUI 的内置动画系统

---

## 参考资源

### 设计系统

- [Material Design](https://material.io/design)
- [Apple Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/)
- [Tailwind CSS](https://tailwindcss.com/)

### 可访问性

- [WCAG 2.1 Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
- [WebAIM Contrast Checker](https://webaim.org/resources/contrastchecker/)

### 动画

- [Easing Functions](https://easings.net/)
- [Material Motion](https://material.io/design/motion/)

---

## 常见问题

### Q: 如何自定义颜色？

A: 可以通过修改主题文件（`themes/*.json`）来自定义基础颜色，语义化颜色会自动适配。

### Q: 如何添加新的语义化颜色？

A: 在 `SemanticColors` 结构体中添加新字段，并在 `from_theme` 方法中初始化。

### Q: 动画效果不流畅怎么办？

A: 检查是否使用了 GPU 加速的属性，避免在动画中修改布局属性。

### Q: 响应式布局在某些尺寸下显示异常？

A: 检查断点设置，确保所有尺寸都有对应的布局逻辑。

---

## 更新日志

### 2026-02-20

- ✅ 创建视觉优化系统
- ✅ 定义语义化颜色
- ✅ 定义视觉层次工具
- ✅ 定义动画和响应式工具
- ✅ 编写完整使用指南

---

## 贡献

欢迎提交改进建议和 Pull Request！

如有问题，请查看：
- 完整优化方案: `claude_优化.md`
- 优化进度: `OPTIMIZATION_PROGRESS.md`
- 错误处理指南: `ERROR_HANDLING_GUIDE.md`
- 快捷键指南: `SHORTCUTS_GUIDE.md`

---

**文档版本**: 1.0.0  
**最后更新**: 2026-02-20  
**维护者**: MyTool Development Team
