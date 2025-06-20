use gpui::{
    AnyElement, App, AppContext, Context, Entity, Hsla, IntoElement, Radians, Render, RenderOnce,
    SharedString, StyleRefinement, Styled, Svg, Transformation, Window,
    prelude::FluentBuilder as _, svg,
};
use gpui_component::{ActiveTheme, Sizable, Size};

#[derive(IntoElement, Clone)]
pub enum IconName {
    ALargeSmall,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Asterisk,
    Bell,
    BookOpen,
    Bot,
    Calendar,
    ChartPie,
    Check,
    ChevronDown,
    ChevronLeft,
    ChevronRight,
    ChevronUp,
    ChevronsUpDown,
    CircleCheck,
    CircleUser,
    CircleX,
    Close,
    Copy,
    Dash,
    Delete,
    Ellipsis,
    EllipsisVertical,
    ExternalLink,
    Eye,
    EyeOff,
    Frame,
    GalleryVerticalEnd,
    GitHub,
    Globe,
    Heart,
    HeartOff,
    Inbox,
    Info,
    Inspector,
    LayoutDashboard,
    Loader,
    LoaderCircle,
    Map,
    Maximize,
    Menu,
    Minimize,
    Minus,
    Moon,
    Palette,
    PanelBottom,
    PanelBottomOpen,
    PanelLeft,
    PanelLeftClose,
    PanelLeftOpen,
    PanelRight,
    PanelRightClose,
    PanelRightOpen,
    Plus,
    ResizeCorner,
    Search,
    Settings,
    Settings2,
    SortAscending,
    SortDescending,
    SquareTerminal,
    Star,
    StarOff,
    Sun,
    ThumbsDown,
    ThumbsUp,
    TriangleAlert,
    WindowClose,
    WindowMaximize,
    WindowMinimize,
    WindowRestore,
    // planify icons
    AlarmSymbolic,
    ArrowCircularTopRightSymbolic,
    ArrowTurnDownRightSymbolic,
    Arrow3DownSymbolic,
    Arrow3RightSymbolic,
    CarouselSymbolic,
    ChatBubbleTextSymbolic,
    CheckRoundOutlineSymbolic,
    CheckRoundOutlineWholeSymbolic,
    CheckmarkSmallSymbolic,
    ClipboardSymbolic,
    ClockSymbolic,
    CloudOutlineThickSymbolic,
    Cloud,
    ColorSymbolic,
    CrossLargeCircleFilledSymbolic,
    CrossLargeCircleOutlineSymbolic,
    DelaySymbolic,
    DockLeftSymbolic,
    DockRightSymbolic,
    EditFindSymbolic,
    EditSymbolic,
    ExternalLinkSymbolic,
    EyeOpenNegativeFilledSymbolic,
    FlagOutlineThickSymbolic,
    FolderDownloadSymbolic,
    FunnelOutlineSymbolic,
    GoNextSymbolic,
    GoUpSymbolic,
    GridLargeSymbolic,
    HeartOutlineThickSymbolic,
    InfoOutlineSymbolic,
    ListDragHandleSymbolic,
    ListLargeSymbolic,
    ListSymbolic,
    MailAttachmentSymbolic,
    MailSymbolic,
    MailboxSymbolic,
    MenuLargeSymbolic,
    MonthSymbolic,
    NavigateSymbolic,
    PaperSymbolic,
    PermissionsGenericSymbolic,
    PinSymbolic,
    PlaylistRepeatSymbolic,
    PlusLargeSymbolic,
    ProcessErrorSymbolic,
    ReactionAdd2Symbolic,
    RescueSymbolic,
    RotationEditSymbolic,
    SettingsSymbolic,
    ShareAltSymbolic,
    ShieldSafeSymbolic,
    ShoeBoxSymbolic,
    SizeVerticallySymbolic,
    StarOutlineThickSymbolic,
    StepOutSymbolic,
    TabsStackSymbolic,
    TagOutlineAddSymbolic,
    TagOutlineRemoveSymbolic,
    TagOutlineSymbolic,
    TextJustifyLeftSymbolic,
    TodayCalendarSymbolic,
    Todoist,
    UpdateSymbolic,
    UserTrashSymbolic,
    VerticalArrowsLongSymbolic,
    ViewColumnsSymbolic,
    ViewListOrderedSymbolic,
    ViewMoreSymbolic,
    ViewSortDescendingRtlSymbolic,
    WorkWeekSymbolic,
}

impl IconName {
    pub fn path(self) -> SharedString {
        match self {
            Self::ALargeSmall => "icons/a-large-small.svg",
            Self::ArrowDown => "icons/arrow-down.svg",
            Self::ArrowLeft => "icons/arrow-left.svg",
            Self::ArrowRight => "icons/arrow-right.svg",
            Self::ArrowUp => "icons/arrow-up.svg",
            Self::Asterisk => "icons/asterisk.svg",
            Self::Bell => "icons/bell.svg",
            Self::BookOpen => "icons/book-open.svg",
            Self::Bot => "icons/bot.svg",
            Self::Calendar => "icons/calendar.svg",
            Self::ChartPie => "icons/chart-pie.svg",
            Self::Check => "icons/check.svg",
            Self::ChevronDown => "icons/chevron-down.svg",
            Self::ChevronLeft => "icons/chevron-left.svg",
            Self::ChevronRight => "icons/chevron-right.svg",
            Self::ChevronUp => "icons/chevron-up.svg",
            Self::ChevronsUpDown => "icons/chevrons-up-down.svg",
            Self::CircleCheck => "icons/circle-check.svg",
            Self::CircleUser => "icons/circle-user.svg",
            Self::CircleX => "icons/circle-x.svg",
            Self::Close => "icons/close.svg",
            Self::Copy => "icons/copy.svg",
            Self::Dash => "icons/dash.svg",
            Self::Delete => "icons/delete.svg",
            Self::Ellipsis => "icons/ellipsis.svg",
            Self::EllipsisVertical => "icons/ellipsis-vertical.svg",
            Self::ExternalLink => "icons/external-link.svg",
            Self::Eye => "icons/eye.svg",
            Self::EyeOff => "icons/eye-off.svg",
            Self::Frame => "icons/frame.svg",
            Self::GalleryVerticalEnd => "icons/gallery-vertical-end.svg",
            Self::GitHub => "icons/github.svg",
            Self::Globe => "icons/globe.svg",
            Self::Heart => "icons/heart.svg",
            Self::HeartOff => "icons/heart-off.svg",
            Self::Inbox => "icons/inbox.svg",
            Self::Info => "icons/info.svg",
            Self::Inspector => "icons/inspector.svg",
            Self::LayoutDashboard => "icons/layout-dashboard.svg",
            Self::Loader => "icons/loader.svg",
            Self::LoaderCircle => "icons/loader-circle.svg",
            Self::Map => "icons/map.svg",
            Self::Maximize => "icons/maximize.svg",
            Self::Menu => "icons/menu.svg",
            Self::Minimize => "icons/minimize.svg",
            Self::Minus => "icons/minus.svg",
            Self::Moon => "icons/moon.svg",
            Self::Palette => "icons/palette.svg",
            Self::PanelBottom => "icons/panel-bottom.svg",
            Self::PanelBottomOpen => "icons/panel-bottom-open.svg",
            Self::PanelLeft => "icons/panel-left.svg",
            Self::PanelLeftClose => "icons/panel-left-close.svg",
            Self::PanelLeftOpen => "icons/panel-left-open.svg",
            Self::PanelRight => "icons/panel-right.svg",
            Self::PanelRightClose => "icons/panel-right-close.svg",
            Self::PanelRightOpen => "icons/panel-right-open.svg",
            Self::Plus => "icons/plus.svg",
            Self::ResizeCorner => "icons/resize-corner.svg",
            Self::Search => "icons/search.svg",
            Self::Settings => "icons/settings.svg",
            Self::Settings2 => "icons/settings-2.svg",
            Self::SortAscending => "icons/sort-ascending.svg",
            Self::SortDescending => "icons/sort-descending.svg",
            Self::SquareTerminal => "icons/square-terminal.svg",
            Self::Star => "icons/star.svg",
            Self::StarOff => "icons/star-off.svg",
            Self::Sun => "icons/sun.svg",
            Self::ThumbsDown => "icons/thumbs-down.svg",
            Self::ThumbsUp => "icons/thumbs-up.svg",
            Self::TriangleAlert => "icons/triangle-alert.svg",
            Self::WindowClose => "icons/window-close.svg",
            Self::WindowMaximize => "icons/window-maximize.svg",
            Self::WindowMinimize => "icons/window-minimize.svg",
            Self::WindowRestore => "icons/window-restore.svg",
            // planify icons
            Self::AlarmSymbolic => "planify-icons/alarm-symbolic.svg",
            Self::ArrowCircularTopRightSymbolic => {
                "planify-icons/arrow-circular-top-right-symbolic.svg"
            }
            Self::ArrowTurnDownRightSymbolic => "planify-icons/arrow-turn-down-right-symbolic.svg",
            Self::Arrow3DownSymbolic => "planify-icons/arrow3-down-symbolic.svg",
            Self::Arrow3RightSymbolic => "planify-icons/arrow3-right-symbolic.svg",
            Self::CarouselSymbolic => "planify-icons/carousel-symbolic.svg",
            Self::ChatBubbleTextSymbolic => "planify-icons/chat-bubble-text-symbolic.svg",
            Self::CheckRoundOutlineSymbolic => "planify-icons/check-round-outline-symbolic.svg",
            Self::CheckRoundOutlineWholeSymbolic => {
                "planify-icons/check-round-outline-whole-symbolic.svg"
            }
            Self::CheckmarkSmallSymbolic => "planify-icons/checkmark-small-symbolic.svg",
            Self::ClipboardSymbolic => "planify-icons/clipboard-symbolic.svg",
            Self::ClockSymbolic => "planify-icons/clock-symbolic.svg",
            Self::CloudOutlineThickSymbolic => "planify-icons/cloud-outline-thick-symbolic.svg",
            Self::Cloud => "planify-icons/cloud-symbolic.svg",
            Self::ColorSymbolic => "planify-icons/color-symbolic.svg",
            Self::CrossLargeCircleFilledSymbolic => {
                "planify-icons/cross-large-circle-filled-symbolic.svg"
            }
            Self::CrossLargeCircleOutlineSymbolic => {
                "planify-icons/cross-large-circle-outline-symbolic.svg"
            }
            Self::DelaySymbolic => "planify-icons/delay-symbolic.svg",
            Self::DockLeftSymbolic => "planify-icons/dock-left-symbolic.svg",
            Self::DockRightSymbolic => "planify-icons/dock-right-symbolic.svg",
            Self::EditFindSymbolic => "planify-icons/edit-find-symbolic.svg",
            Self::EditSymbolic => "planify-icons/edit-symbolic.svg",
            Self::ExternalLinkSymbolic => "planify-icons/external-link-symbolic.svg",
            Self::EyeOpenNegativeFilledSymbolic => {
                "planify-icons/eye-open-negative-filled-symbolic.svg"
            }
            Self::FlagOutlineThickSymbolic => "planify-icons/flag-outline-thick-symbolic.svg",
            Self::FolderDownloadSymbolic => "planify-icons/folder-download-symbolic.svg",
            Self::FunnelOutlineSymbolic => "planify-icons/funnel-outline-symbolic.svg",
            Self::GoNextSymbolic => "planify-icons/go-next-symbolic.svg",
            Self::GoUpSymbolic => "planify-icons/go-up-symbolic.svg",
            Self::GridLargeSymbolic => "planify-icons/grid-large-symbolic.svg",
            Self::HeartOutlineThickSymbolic => "planify-icons/heart-outline-thick-symbolic.svg",
            Self::InfoOutlineSymbolic => "planify-icons/info-outline-symbolic.svg",
            Self::ListDragHandleSymbolic => "planify-icons/list-drag-handle-symbolic.svg",
            Self::ListLargeSymbolic => "planify-icons/list-large-symbolic.svg",
            Self::ListSymbolic => "planify-icons/list-symbolic.svg",
            Self::MailAttachmentSymbolic => "planify-icons/mail-attachment-symbolic.svg",
            Self::MailSymbolic => "planify-icons/mail-symbolic.svg",
            Self::MailboxSymbolic => "planify-icons/mailbox-symbolic.svg",
            Self::MenuLargeSymbolic => "planify-icons/menu-large-symbolic.svg",
            Self::MonthSymbolic => "planify-icons/month-symbolic.svg",
            Self::NavigateSymbolic => "planify-icons/navigate-symbolic.svg",
            Self::PaperSymbolic => "planify-icons/paper-symbolic.svg",
            Self::PermissionsGenericSymbolic => "planify-icons/permissions-generic-symbolic.svg",
            Self::PinSymbolic => "planify-icons/pin-symbolic.svg",
            Self::PlaylistRepeatSymbolic => "planify-icons/playlist-repeat-symbolic.svg",
            Self::PlusLargeSymbolic => "planify-icons/plus-large-symbolic.svg",
            Self::ProcessErrorSymbolic => "planify-icons/process-error-symbolic.svg",
            Self::ReactionAdd2Symbolic => "planify-icons/reaction-add2-symbolic.svg",
            Self::RescueSymbolic => "planify-icons/rescue-symbolic.svg",
            Self::RotationEditSymbolic => "planify-icons/rotation-edit-symbolic.svg",
            Self::SettingsSymbolic => "planify-icons/settings-symbolic.svg",
            Self::ShareAltSymbolic => "planify-icons/share-alt-symbolic.svg",
            Self::ShieldSafeSymbolic => "planify-icons/shield-safe-symbolic.svg",
            Self::ShoeBoxSymbolic => "planify-icons/shoe-box-symbolic.svg",
            Self::SizeVerticallySymbolic => "planify-icons/size-vertically-symbolic.svg",
            Self::StarOutlineThickSymbolic => "planify-icons/star-outline-thick-symbolic.svg",
            Self::StepOutSymbolic => "planify-icons/step-out-symbolic.svg",
            Self::TabsStackSymbolic => "planify-icons/tabs-stack-symbolic.svg",
            Self::TagOutlineAddSymbolic => "planify-icons/tag-outline-add-symbolic.svg",
            Self::TagOutlineRemoveSymbolic => "planify-icons/tag-outline-remove-symbolic.svg",
            Self::TagOutlineSymbolic => "planify-icons/tag-outline-symbolic.svg",
            Self::TextJustifyLeftSymbolic => "planify-icons/text-justify-left-symbolic.svg",
            Self::TodayCalendarSymbolic => "planify-icons/today-calendar-symbolic.svg",
            Self::Todoist => "planify-icons/todoist-symbolic.svg",
            Self::UpdateSymbolic => "planify-icons/update-symbolic.svg",
            Self::UserTrashSymbolic => "planify-icons/user-trash-symbolic.svg",
            Self::VerticalArrowsLongSymbolic => "planify-icons/vertical-arrows-long-symbolic.svg",
            Self::ViewColumnsSymbolic => "planify-icons/view-columns-symbolic.svg",
            Self::ViewListOrderedSymbolic => "planify-icons/view-list-ordered-symbolic.svg",
            Self::ViewMoreSymbolic => "planify-icons/view-more-symbolic.svg",
            Self::ViewSortDescendingRtlSymbolic => {
                "planify-icons/view-sort-descending-rtl-symbolic.svg"
            }
            Self::WorkWeekSymbolic => "planify-icons/work-week-symbolic.svg",
        }
        .into()
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "a-large-small" => Self::ALargeSmall,
            "arrow-down" => Self::ArrowDown,
            "arrow-left" => Self::ArrowLeft,
            "arrow-right" => Self::ArrowRight,
            "arrow-up" => Self::ArrowUp,
            "asterisk" => Self::Asterisk,
            "bell" => Self::Bell,
            "book-open" => Self::BookOpen,
            "bot" => Self::Bot,
            "calendar" => Self::Calendar,
            "chart-pie" => Self::ChartPie,
            "check" => Self::Check,
            "chevron-down" => Self::ChevronDown,
            "chevron-left" => Self::ChevronLeft,
            "chevron-right" => Self::ChevronRight,
            "chevron-up" => Self::ChevronUp,
            "chevrons-up-down" => Self::ChevronsUpDown,
            "circle-check" => Self::CircleCheck,
            "circle-user" => Self::CircleUser,
            "circle-x" => Self::CircleX,
            "close" => Self::Close,
            "copy" => Self::Copy,
            "dash" => Self::Dash,
            "delete" => Self::Delete,
            "ellipsis" => Self::Ellipsis,
            "ellipsis-vertical" => Self::EllipsisVertical,
            "external-link" => Self::ExternalLink,
            "eye" => Self::Eye,
            "eye-off" => Self::EyeOff,
            "frame" => Self::Frame,
            "gallery-vertical-end" => Self::GalleryVerticalEnd,
            "github" => Self::GitHub,
            "globe" => Self::Globe,
            "heart" => Self::Heart,
            "heart-off" => Self::HeartOff,
            "inbox" => Self::Inbox,
            "info" => Self::Info,
            "inspector" => Self::Inspector,
            "layout-dashboard" => Self::LayoutDashboard,
            "loader" => Self::Loader,
            "loader-circle" => Self::LoaderCircle,
            "map" => Self::Map,
            "maximize" => Self::Maximize,
            "menu" => Self::Menu,
            "minimize" => Self::Minimize,
            "minus" => Self::Minus,
            "moon" => Self::Moon,
            "palette" => Self::Palette,
            "panel-bottom" => Self::PanelBottom,
            "panel-bottom-open" => Self::PanelBottomOpen,
            "panel-left" => Self::PanelLeft,
            "panel-left-close" => Self::PanelLeftClose,
            "panel-left-open" => Self::PanelLeftOpen,
            "panel-right" => Self::PanelRight,
            "panel-right-close" => Self::PanelRightClose,
            "panel-right-open" => Self::PanelRightOpen,
            "plus" => Self::Plus,
            "resize-corner" => Self::ResizeCorner,
            "search" => Self::Search,
            "settings" => Self::Settings,
            "settings-2" => Self::Settings2,
            "sort-ascending" => Self::SortAscending,
            "sort-descending" => Self::SortDescending,
            "square-terminal" => Self::SquareTerminal,
            "star" => Self::Star,
            "star-off" => Self::StarOff,
            "sun" => Self::Sun,
            "thumbs-down" => Self::ThumbsDown,
            "thumbs-up" => Self::ThumbsUp,
            "triangle-alert" => Self::TriangleAlert,
            "window-close" => Self::WindowClose,
            "window-maximize" => Self::WindowMaximize,
            "window-minimize" => Self::WindowMinimize,
            "window-restore" => Self::WindowRestore,
            // planify icons
            "alarm-symbolic" => Self::AlarmSymbolic,
            "arrow-circular-top-right-symbolic" => Self::ArrowCircularTopRightSymbolic,
            "arrow-turn-down-right-symbolic" => Self::ArrowTurnDownRightSymbolic,
            "arrow3-down-symbolic" => Self::Arrow3DownSymbolic,
            "arrow3-right-symbolic" => Self::Arrow3RightSymbolic,
            "carousel-symbolic" => Self::CarouselSymbolic,
            "chat-bubble-text-symbolic" => Self::ChatBubbleTextSymbolic,
            "check-round-outline-symbolic" => Self::CheckRoundOutlineSymbolic,
            "check-round-outline-whole-symbolic" => Self::CheckRoundOutlineWholeSymbolic,
            "checkmark-small-symbolic" => Self::CheckmarkSmallSymbolic,
            "clipboard-symbolic" => Self::ClipboardSymbolic,
            "clock-symbolic" => Self::ClockSymbolic,
            "cloud-outline-thick-symbolic" => Self::CloudOutlineThickSymbolic,
            "cloud-symbolic" => Self::Cloud,
            "color-symbolic" => Self::ColorSymbolic,
            "cross-large-circle-filled-symbolic" => Self::CrossLargeCircleFilledSymbolic,
            "cross-large-circle-outline-symbolic" => Self::CrossLargeCircleOutlineSymbolic,
            "delay-symbolic" => Self::DelaySymbolic,
            "dock-left-symbolic" => Self::DockLeftSymbolic,
            "dock-right-symbolic" => Self::DockRightSymbolic,
            "edit-find-symbolic" => Self::EditFindSymbolic,
            "edit-symbolic" => Self::EditSymbolic,
            "external-link-symbolic" => Self::ExternalLinkSymbolic,
            "eye-open-negative-filled-symbolic" => Self::EyeOpenNegativeFilledSymbolic,
            "flag-outline-thick-symbolic" => Self::FlagOutlineThickSymbolic,
            "folder-download-symbolic" => Self::FolderDownloadSymbolic,
            "funnel-outline-symbolic" => Self::FunnelOutlineSymbolic,
            "go-next-symbolic" => Self::GoNextSymbolic,
            "go-up-symbolic" => Self::GoUpSymbolic,
            "grid-large-symbolic" => Self::GridLargeSymbolic,
            "heart-outline-thick-symbolic" => Self::HeartOutlineThickSymbolic,
            "info-outline-symbolic" => Self::InfoOutlineSymbolic,
            "list-drag-handle-symbolic" => Self::ListDragHandleSymbolic,
            "list-large-symbolic" => Self::ListLargeSymbolic,
            "list-symbolic" => Self::ListSymbolic,
            "mail-attachment-symbolic" => Self::MailAttachmentSymbolic,
            "mail-symbolic" => Self::MailSymbolic,
            "mailbox-symbolic" => Self::MailboxSymbolic,
            "menu-large-symbolic" => Self::MenuLargeSymbolic,
            "month-symbolic" => Self::MonthSymbolic,
            "navigate-symbolic" => Self::NavigateSymbolic,
            "paper-symbolic" => Self::PaperSymbolic,
            "permissions-generic-symbolic" => Self::PermissionsGenericSymbolic,
            "pin-symbolic" => Self::PinSymbolic,
            "playlist-repeat-symbolic" => Self::PlaylistRepeatSymbolic,
            "plus-large-symbolic" => Self::PlusLargeSymbolic,
            "process-error-symbolic" => Self::ProcessErrorSymbolic,
            "reaction-add2-symbolic" => Self::ReactionAdd2Symbolic,
            "rescue-symbolic" => Self::RescueSymbolic,
            "rotation-edit-symbolic" => Self::RotationEditSymbolic,
            "settings-symbolic" => Self::SettingsSymbolic,
            "share-alt-symbolic" => Self::ShareAltSymbolic,
            "shield-safe-symbolic" => Self::ShieldSafeSymbolic,
            "shoe-box-symbolic" => Self::ShoeBoxSymbolic,
            "size-vertically-symbolic" => Self::SizeVerticallySymbolic,
            "star-outline-thick-symbolic" => Self::StarOutlineThickSymbolic,
            "step-out-symbolic" => Self::StepOutSymbolic,
            "tabs-stack-symbolic" => Self::TabsStackSymbolic,
            "tag-outline-add-symbolic" => Self::TagOutlineAddSymbolic,
            "tag-outline-remove-symbolic" => Self::TagOutlineRemoveSymbolic,
            "tag-outline-symbolic" => Self::TagOutlineSymbolic,
            "text-justify-left-symbolic" => Self::TextJustifyLeftSymbolic,
            "today-calendar-symbolic" => Self::TodayCalendarSymbolic,
            "todoist-symbolic" => Self::Todoist,
            "update-symbolic" => Self::UpdateSymbolic,
            "user-trash-symbolic" => Self::UserTrashSymbolic,
            "vertical-arrows-long-symbolic" => Self::VerticalArrowsLongSymbolic,
            "view-columns-symbolic" => Self::ViewColumnsSymbolic,
            "view-list-ordered-symbolic" => Self::ViewListOrderedSymbolic,
            "view-more-symbolic" => Self::ViewMoreSymbolic,
            "view-sort-descending-rtl-symbolic" => Self::ViewSortDescendingRtlSymbolic,
            "work-week-symbolic" => Self::WorkWeekSymbolic,
            _ => Self::from_str("todoist-symbolic"), // Default to Todoist icon if not found
        }
    }
    /// Return the icon as a Entity<MyIcon>
    pub fn view(self, cx: &mut App) -> Entity<Icon> {
        Icon::build(self).view(cx)
    }
}

impl From<IconName> for Icon {
    fn from(val: IconName) -> Self {
        Icon::build(val)
    }
}

impl From<IconName> for AnyElement {
    fn from(val: IconName) -> Self {
        Icon::build(val).into_any_element()
    }
}

impl RenderOnce for IconName {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        Icon::build(self)
    }
}

#[derive(IntoElement)]
pub struct Icon {
    base: Svg,
    style: StyleRefinement,
    path: SharedString,
    text_color: Option<Hsla>,
    size: Option<Size>,
    rotation: Option<Radians>,
}

impl Default for Icon {
    fn default() -> Self {
        Self {
            base: svg().flex_none().size_4(),
            style: StyleRefinement::default(),
            path: "".into(),
            text_color: None,
            size: None,
            rotation: None,
        }
    }
}

impl Clone for Icon {
    fn clone(&self) -> Self {
        let mut this = Self::default().path(self.path.clone());
        this.style = self.style.clone();
        this.rotation = self.rotation;
        this.size = self.size;
        this.text_color = self.text_color;
        this
    }
}

pub trait IconNamed {
    fn path(&self) -> SharedString;
}

impl Icon {
    pub fn new(icon: impl Into<Icon>) -> Self {
        icon.into()
    }

    fn build(name: IconName) -> Self {
        Self::default().path(name.path())
    }

    /// Set the icon path of the Assets bundle
    ///
    /// For example: `icons/foo.svg`
    pub fn path(mut self, path: impl Into<SharedString>) -> Self {
        self.path = path.into();
        self
    }

    /// Create a new view for the icon
    pub fn view(self, cx: &mut App) -> Entity<Icon> {
        cx.new(|_| self)
    }

    pub fn transform(mut self, transformation: gpui::Transformation) -> Self {
        self.base = self.base.with_transformation(transformation);
        self
    }

    pub fn empty() -> Self {
        Self::default()
    }

    /// Rotate the icon by the given angle
    pub fn rotate(mut self, radians: impl Into<Radians>) -> Self {
        self.base = self
            .base
            .with_transformation(Transformation::rotate(radians));
        self
    }
}

impl Styled for Icon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }

    fn text_color(mut self, color: impl Into<Hsla>) -> Self {
        self.text_color = Some(color.into());
        self
    }
}

impl Sizable for Icon {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let text_color = self.text_color.unwrap_or_else(|| window.text_style().color);
        let text_size = window.text_style().font_size.to_pixels(window.rem_size());
        let has_base_size = self.style.size.width.is_some() || self.style.size.height.is_some();

        let mut base = self.base;
        *base.style() = self.style;

        base.flex_shrink_0()
            .text_color(text_color)
            .when(!has_base_size, |this| this.size(text_size))
            .when_some(self.size, |this, size| match size {
                Size::Size(px) => this.size(px),
                Size::XSmall => this.size_3(),
                Size::Small => this.size_3p5(),
                Size::Medium => this.size_4(),
                Size::Large => this.size_6(),
            })
            .path(self.path)
    }
}

impl From<Icon> for AnyElement {
    fn from(val: Icon) -> Self {
        val.into_any_element()
    }
}

impl Render for Icon {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let text_color = self.text_color.unwrap_or_else(|| cx.theme().foreground);
        let text_size = window.text_style().font_size.to_pixels(window.rem_size());
        let has_base_size = self.style.size.width.is_some() || self.style.size.height.is_some();

        let mut base = svg().flex_none();
        *base.style() = self.style.clone();

        base.flex_shrink_0()
            .text_color(text_color)
            .when(!has_base_size, |this| this.size(text_size))
            .when_some(self.size, |this, size| match size {
                Size::Size(px) => this.size(px),
                Size::XSmall => this.size_3(),
                Size::Small => this.size_3p5(),
                Size::Medium => this.size_4(),
                Size::Large => this.size_6(),
            })
            .path(self.path.clone())
            .when_some(self.rotation, |this, rotation| {
                this.with_transformation(Transformation::rotate(rotation))
            })
    }
}
