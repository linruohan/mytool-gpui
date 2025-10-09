use std::{ops::Range, rc::Rc};

use crate::{DBState, get_projects};
use gpui::{
    Action, AnyElement, App, AppContext, ClickEvent, Context, Entity, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, TextAlign, Window, div, prelude::FluentBuilder as _,
};
use gpui_component::{
    ActiveTheme as _, Sizable as _, Size, StyleSized as _, StyledExt,
    button::Button,
    h_flex,
    indicator::Indicator,
    input::{InputEvent, InputState, TextInput},
    label::Label,
    popup_menu::{PopupMenu, PopupMenuExt},
    table::{Column, ColumnFixed, ColumnSort, Table, TableDelegate, TableEvent},
    v_flex,
};
use serde::Deserialize;
use todos::entity::ProjectModel;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = table_project, no_json)]
struct ChangeSize(Size);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = table_project, no_json)]
struct OpenDetail(usize);

struct StockTableDelegate {
    _projects: Vec<Rc<ProjectModel>>,
    matched_projects: Vec<Rc<ProjectModel>>,
    query: SharedString, //搜索project
    columns: Vec<Column>,
    size: Size,
    loading: bool,
    eof: bool,
    visible_rows: Range<usize>,
    visible_cols: Range<usize>,
}

impl StockTableDelegate {
    fn new() -> Self {
        Self {
            size: Size::default(),
            _projects: Vec::new(),
            matched_projects: Vec::new(),
            query: "".into(),
            columns: vec![
                Column::new("id", "ID")
                    .width(60.)
                    .fixed(ColumnFixed::Left)
                    .sortable()
                    .resizable(false),
                Column::new("name", "名字")
                    .width(60.)
                    .fixed(ColumnFixed::Left)
                    .resizable(false)
                    .sortable()
                    .text_right()
                    .p_0(),
                Column::new("color", "颜色")
                    .width(60.)
                    .sortable()
                    .fixed(ColumnFixed::Left),
                Column::new("emoji", "Chg").text_right().p_0(),
                Column::new("description", "描述").text_right().p_0(),
            ],
            loading: false,
            eof: false,
            visible_cols: Range::default(),
            visible_rows: Range::default(),
        }
    }
    fn on_search(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        let companies: Vec<Rc<ProjectModel>> = self
            ._projects
            .iter()
            .filter(|menu| {
                menu.name
                    .to_lowercase()
                    .contains(&self.query.to_lowercase())
            })
            .cloned()
            .collect();
        self.matched_projects = companies;
    }
    fn update_projects(&mut self, menus: Vec<Rc<ProjectModel>>) {
        self._projects = menus;
        self.matched_projects = self._projects.clone();
    }
    #[allow(unused)]
    fn render_percent(&self, col: &Column, val: f64, cx: &mut Context<Table<Self>>) -> AnyElement {
        let right_num = ((val - val.floor()) * 1000.).floor() as i32;

        div()
            .h_full()
            .table_cell_size(self.size)
            .when(col.align == TextAlign::Right, |this| {
                this.h_flex().justify_end()
            })
            .map(|this| {
                if right_num % 3 == 0 {
                    this.text_color(cx.theme().red)
                        .bg(cx.theme().red_light.alpha(0.05))
                } else if right_num % 3 == 1 {
                    this.text_color(cx.theme().green)
                        .bg(cx.theme().green_light.alpha(0.05))
                } else {
                    this
                }
            })
            .child(format!("{:.2}%", val * 100.))
            .into_any_element()
    }
    #[allow(unused)]
    fn render_value_cell(
        &self,
        col: &Column,
        val: f64,
        cx: &mut Context<Table<Self>>,
    ) -> AnyElement {
        let this = div()
            .h_full()
            .table_cell_size(self.size)
            .child(format!("{:.3}", val));
        // Val is a 0.0 .. n.0
        // 30% to red, 30% to green, others to default
        let right_num = ((val - val.floor()) * 1000.).floor() as i32;

        let this = if right_num % 3 == 0 {
            this.text_color(cx.theme().red)
                .bg(cx.theme().red_light.alpha(0.05))
        } else if right_num % 3 == 1 {
            this.text_color(cx.theme().green)
                .bg(cx.theme().green_light.alpha(0.05))
        } else {
            this
        };

        this.when(col.align == TextAlign::Right, |this| {
            this.h_flex().justify_end()
        })
        .into_any_element()
    }
}

impl TableDelegate for StockTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.matched_projects.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> &Column {
        &self.columns[col_ix]
    }

    fn render_th(
        &self,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<Table<Self>>,
    ) -> impl IntoElement {
        let col = self.columns.get(col_ix).unwrap();

        div()
            .child(col.name.clone())
            .when(col_ix >= 3 && col_ix <= 10, |this| {
                this.table_cell_size(self.size)
            })
            .when(col.align == TextAlign::Right, |this| {
                this.h_flex().w_full().justify_end()
            })
    }

    fn context_menu(
        &self,
        row_ix: usize,
        menu: PopupMenu,
        _window: &Window,
        _cx: &App,
    ) -> PopupMenu {
        menu.menu(
            format!("Selected Row: {}", row_ix),
            Box::new(OpenDetail(row_ix)),
        )
        .separator()
        .menu("Size Large", Box::new(ChangeSize(Size::Large)))
        .menu("Size Medium", Box::new(ChangeSize(Size::Medium)))
        .menu("Size Small", Box::new(ChangeSize(Size::Small)))
        .menu("Size XSmall", Box::new(ChangeSize(Size::XSmall)))
    }

    fn render_tr(
        &self,
        row_ix: usize,
        _: &mut Window,
        cx: &mut Context<Table<Self>>,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(row_ix)
            .on_click(cx.listener(|_, ev: &ClickEvent, _, _| {
                println!(
                    "You have clicked row with secondary: {}",
                    ev.modifiers().secondary()
                )
            }))
    }

    /// 渲染td，真实的字渲染
    fn render_td(
        &self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<Table<Self>>,
    ) -> impl IntoElement {
        let project = self.matched_projects.get(row_ix).unwrap();
        let col = self.columns.get(col_ix).unwrap();

        match col.key.as_ref() {
            "id" => project.id.clone().into_any_element(),
            // 渲染带颜色
            "name" => div()
                .map(|this| {
                    if project.name.contains("UVP") {
                        this.text_color(cx.theme().blue)
                    } else {
                        this.text_color(cx.theme().magenta)
                    }
                })
                .child(project.name.clone())
                .into_any_element(),
            "color" => project
                .color
                .clone()
                .unwrap_or_default()
                .clone()
                .into_any_element(),
            "emoji" => project
                .emoji
                .clone()
                .unwrap_or_default()
                .clone()
                .into_any_element(),
            "description" => project
                .description
                .clone()
                .unwrap_or_default()
                .clone()
                .into_any_element(),
            // "price" => self.render_value_cell(&col, project.price, cx),
            // "change_percent" => self.render_percent(&col, project.change_percent, cx),
            _ => "--".to_string().into_any_element(),
        }
    }

    fn move_column(
        &mut self,
        col_ix: usize,
        to_ix: usize,
        _: &mut Window,
        _: &mut Context<Table<Self>>,
    ) {
        let col = self.columns.remove(col_ix);
        self.columns.insert(to_ix, col);
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        _: &mut Context<Table<Self>>,
    ) {
        if let Some(col) = self.columns.get_mut(col_ix) {
            match col.key.as_ref() {
                "id" => self.matched_projects.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b.id.cmp(&a.id),
                    _ => a.id.cmp(&b.id),
                }),
                "name" => self.matched_projects.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b.name.cmp(&a.name),
                    _ => a.id.cmp(&b.id),
                }),
                "color" => self.matched_projects.sort_by(|a, b| {
                    let cmp = match (a.color.as_ref(), b.color.as_ref()) {
                        (Some(a_color), Some(b_color)) => a_color.partial_cmp(b_color),
                        (None, Some(_)) => Some(std::cmp::Ordering::Greater),
                        (Some(_), None) => Some(std::cmp::Ordering::Less),
                        (None, None) => Some(std::cmp::Ordering::Equal),
                    };

                    match (sort, cmp) {
                        (ColumnSort::Descending, Some(ordering)) => ordering.reverse(),
                        (_, Some(ordering)) => ordering,
                        (_, None) => std::cmp::Ordering::Equal, // Handle partial_cmp returning None
                    }
                }),
                _ => {}
            }
        }
    }

    fn visible_rows_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _: &mut Context<Table<Self>>,
    ) {
        self.visible_rows = visible_range;
    }

    fn visible_columns_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _: &mut Context<Table<Self>>,
    ) {
        self.visible_cols = visible_range;
    }
}

pub struct TableStory {
    table: Entity<Table<StockTableDelegate>>,
    num_stocks_input: Entity<InputState>,
    size: Size,
}

impl super::Mytool for TableStory {
    fn title() -> &'static str {
        "Project table"
    }

    fn description() -> &'static str {
        "A complex data table with selection, sorting, column moving, and loading more."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn closable() -> bool {
        false
    }
}

impl Focusable for TableStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.table.focus_handle(cx)
    }
}

impl TableStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Create the number input field with validation for positive integers
        let num_stocks_input = cx.new(|cx| {
            let mut input = InputState::new(window, cx)
                .placeholder("search")
                .validate(|s, _| s.parse::<String>().is_ok());
            input.set_value("", window, cx);
            input
        });

        let table = cx.new(|cx| Table::new(StockTableDelegate::new(), window, cx));

        cx.subscribe_in(&table, window, Self::on_table_event)
            .detach();
        cx.subscribe_in(&num_stocks_input, window, Self::on_search_input_change)
            .detach();

        let table_clone = table.clone();
        let db = cx.global::<DBState>().conn.clone();
        cx.spawn(async move |_view, cx| {
            let db = db.lock().await;
            let projects = get_projects(db.clone()).await;
            let rc_projects: Vec<Rc<ProjectModel>> =
                projects.iter().map(|pro| Rc::new(pro.clone())).collect();
            println!("get rc_projects:{}", rc_projects.len());
            let _ = cx
                .update_entity(&table_clone, |list, cx| {
                    list.delegate_mut().update_projects(rc_projects);
                    cx.notify();
                })
                .ok();
        })
        .detach();
        Self {
            table,
            num_stocks_input,
            size: Size::default(),
        }
    }

    // Event handler for changes in the number input field
    fn on_search_input_change(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            // Update when the user presses Enter or the input loses focus
            InputEvent::PressEnter { .. } | InputEvent::Blur => {
                let text = self.num_stocks_input.read(cx).value().clone();
                println!("search: {}", text);
                self.table.update(cx, |table, _| {
                    table.delegate_mut().on_search(text.clone());
                });
                cx.notify();
            }
            _ => {}
        }
    }

    fn on_change_size(&mut self, a: &ChangeSize, _: &mut Window, cx: &mut Context<Self>) {
        self.size = a.0;
        self.table.update(cx, |table, cx| {
            table.set_size(a.0, cx);
            table.delegate_mut().size = a.0;
        });
    }

    fn on_table_event(
        &mut self,
        _: &Entity<Table<StockTableDelegate>>,
        event: &TableEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            TableEvent::ColumnWidthsChanged(col_widths) => {
                println!("Column widths changed: {:?}", col_widths)
            }
            TableEvent::SelectColumn(ix) => println!("Select col: {}", ix),
            TableEvent::DoubleClickedRow(ix) => println!("Double clicked row: {}", ix),
            TableEvent::SelectRow(ix) => println!("Select row: {}", ix),
            TableEvent::MoveColumn(origin_idx, target_idx) => {
                println!("Move col index: {} -> {}", origin_idx, target_idx);
            }
        }
    }
}

impl Render for TableStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let table = &self.table.read(cx);
        let delegate = table.delegate();
        let rows_count = delegate.rows_count(cx);
        let size = self.size;

        v_flex()
            .on_action(cx.listener(Self::on_change_size))
            .size_full()
            .text_sm()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("size")
                            .outline()
                            .small()
                            .label(format!("size: {:?}", self.size))
                            .popup_menu(move |menu, _, _| {
                                menu.menu_with_check(
                                    "Large",
                                    size == Size::Large,
                                    Box::new(ChangeSize(Size::Large)),
                                )
                                .menu_with_check(
                                    "Medium",
                                    size == Size::Medium,
                                    Box::new(ChangeSize(Size::Medium)),
                                )
                                .menu_with_check(
                                    "Small",
                                    size == Size::Small,
                                    Box::new(ChangeSize(Size::Small)),
                                )
                                .menu_with_check(
                                    "XSmall",
                                    size == Size::XSmall,
                                    Box::new(ChangeSize(Size::XSmall)),
                                )
                            }),
                    )
                    .child(
                        Button::new("scroll-top")
                            .outline()
                            .small()
                            .child("Scroll to Top")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.table.update(cx, |table, cx| {
                                    table.scroll_to_row(0, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-bottom")
                            .outline()
                            .small()
                            .child("Scroll to Bottom")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.table.update(cx, |table, cx| {
                                    table.scroll_to_row(table.delegate().rows_count(cx) - 1, cx);
                                })
                            })),
                    ),
            )
            .child(
                h_flex().items_center().gap_2().child(
                    h_flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .child(
                            h_flex()
                                .gap_2()
                                .flex_1()
                                .child(Label::new("Number of Stocks:"))
                                .child(
                                    h_flex()
                                        .min_w_32()
                                        .child(TextInput::new(&self.num_stocks_input).small())
                                        .into_any_element(),
                                )
                                .when(delegate.loading, |this| {
                                    this.child(
                                        h_flex()
                                            .gap_1()
                                            .child(Indicator::new())
                                            .child("Loading..."),
                                    )
                                }),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .child(format!("Total Rows: {}", rows_count))
                                .child(format!("Visible Rows: {:?}", delegate.visible_rows))
                                .child(format!("Visible Cols: {:?}", delegate.visible_cols))
                                .when(delegate.eof, |this| this.child("All data loaded.")),
                        ),
                ),
            )
            .child(self.table.clone())
    }
}
