use std::{ops::Range, sync::LazyLock, time::Duration};

use fake::Fake;
use gpui::{
    div, prelude::FluentBuilder as _, Action, AnyElement, App, AppContext, ClickEvent, Context,
    Entity, Focusable, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, TextAlign, Timer, Window,
};
use gpui_component::{
    button::Button,
    checkbox::Checkbox,
    h_flex,
    indicator::Indicator,
    input::{InputEvent, InputState, TextInput},
    label::Label,
    popup_menu::{PopupMenu, PopupMenuExt},
    table::{Column, ColumnFixed, ColumnSort, Table, TableDelegate, TableEvent},
    v_flex, ActiveTheme as _, Selectable, Sizable as _, Size, StyleSized as _, StyledExt,
};
use serde::{Deserialize, Serialize};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = table_story, no_json)]
struct ChangeSize(Size);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = table_story, no_json)]
struct OpenDetail(usize);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Counter {
    symbol: SharedString,
    market: SharedString,
    name: SharedString,
}

static ALL_COUNTERS: LazyLock<Vec<Counter>> =
    LazyLock::new(|| serde_json::from_str(include_str!("./fixtures/counters.json")).unwrap());

impl Counter {
    fn random() -> Self {
        let len = ALL_COUNTERS.len();
        let ix = rand::random::<usize>() % len;
        ALL_COUNTERS[ix].clone()
    }

    fn symbol_code(&self) -> SharedString {
        format!("{}.{}", self.symbol, self.market).into()
    }
}

#[derive(Clone, Debug, Default)]
struct Stock {
    id: usize,
    counter: Counter,
    price: f64,
    change: f64,
    change_percent: f64,
}

impl Stock {
    fn random_update(&mut self) {
        self.price = (-300.0..999.999).fake::<f64>();
        self.change = (-0.1..5.0).fake::<f64>();
        self.change_percent = (-0.1..0.1).fake::<f64>();
    }
}

fn random_stocks(size: usize) -> Vec<Stock> {
    (0..size)
        .map(|id| Stock {
            id,
            counter: Counter::random(),
            change: (-100.0..100.0).fake(),
            change_percent: (-0.1..0.1).fake(),
            ..Default::default()
        })
        .collect()
}

struct StockTableDelegate {
    stocks: Vec<Stock>,
    columns: Vec<Column>,
    size: Size,
    loading: bool,
    full_loading: bool,
    eof: bool,
    visible_rows: Range<usize>,
    visible_cols: Range<usize>,
}

impl StockTableDelegate {
    fn new(size: usize) -> Self {
        Self {
            size: Size::default(),
            stocks: random_stocks(size),
            columns: vec![
                Column::new("id", "ID")
                    .width(60.)
                    .fixed(ColumnFixed::Left)
                    .resizable(false),
                Column::new("market", "Market")
                    .width(60.)
                    .fixed(ColumnFixed::Left)
                    .resizable(false),
                Column::new("name", "Name")
                    .width(180.)
                    .fixed(ColumnFixed::Left),
                Column::new("symbol", "Symbol")
                    .width(100.)
                    .fixed(ColumnFixed::Left)
                    .sortable(),
                Column::new("price", "Price").sortable().text_right().p_0(),
                Column::new("change", "Chg").sortable().text_right().p_0(),
                Column::new("change_percent", "Chg%")
                    .sortable()
                    .text_right()
                    .p_0(),
            ],
            loading: false,
            full_loading: false,
            eof: false,
            visible_cols: Range::default(),
            visible_rows: Range::default(),
        }
    }

    fn update_stocks(&mut self, size: usize) {
        self.stocks = random_stocks(size);
        self.eof = size <= 50;
        self.loading = false;
        self.full_loading = false;
    }

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
        self.stocks.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> &Column {
        &self.columns[col_ix]
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
                "id" => self.stocks.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b.id.cmp(&a.id),
                    _ => a.id.cmp(&b.id),
                }),
                "symbol" => self.stocks.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b.counter.symbol.cmp(&a.counter.symbol),
                    _ => a.id.cmp(&b.id),
                }),
                "change" | "change_percent" => self.stocks.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b
                        .change
                        .partial_cmp(&a.change)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    _ => a.id.cmp(&b.id),
                }),
                _ => {}
            }
        }
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

    /// NOTE: Performance metrics
    ///
    /// last render 561 cells total: 232.745µs, avg: 414ns
    /// frame duration: 8.825083ms
    ///
    /// This is means render the full table cells takes 232.745µs. Then 232.745µs / 8.82ms = 2.6% of the frame duration.
    ///
    /// If we improve the td rendering, we can reduce the time to render the full table cells.
    fn render_td(
        &self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<Table<Self>>,
    ) -> impl IntoElement {
        let stock = self.stocks.get(row_ix).unwrap();
        let col = self.columns.get(col_ix).unwrap();

        match col.key.as_ref() {
            "id" => stock.id.to_string().into_any_element(),
            "market" => div()
                .map(|this| {
                    if stock.counter.market == "US" {
                        this.text_color(cx.theme().blue)
                    } else {
                        this.text_color(cx.theme().magenta)
                    }
                })
                .child(stock.counter.market.clone())
                .into_any_element(),
            "symbol" => stock.counter.symbol_code().into_any_element(),
            "name" => stock.counter.name.clone().into_any_element(),
            "price" => self.render_value_cell(&col, stock.price, cx),
            "change" => self.render_value_cell(&col, stock.change, cx),
            "change_percent" => self.render_percent(&col, stock.change_percent, cx),
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

    fn loading(&self, _: &App) -> bool {
        self.full_loading
    }

    fn can_load_more(&self, _: &App) -> bool {
        // return !self.loading && !self.eof;
        false
    }

    fn load_more_threshold(&self) -> usize {
        10
    }

    fn load_more(&mut self, _: &mut Window, cx: &mut Context<Table<Self>>) {
        self.loading = true;

        cx.spawn(async move |view, cx| {
            // Simulate network request, delay 1s to load data.
            Timer::after(Duration::from_secs(1)).await;

            cx.update(|cx| {
                let _ = view.update(cx, |view, _| {
                    view.delegate_mut().stocks.extend(random_stocks(2));
                    view.delegate_mut().loading = false;
                    view.delegate_mut().eof = view.delegate().stocks.len() >= 16;
                });
            })
        })
        .detach();
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
    stripe: bool,
    refresh_data: bool,
    size: Size,
}

impl super::Mytool for TableStory {
    fn title() -> &'static str {
        "Table"
    }

    fn description() -> &'static str {
        "A complex data table with selection, sorting, column moving, and loading more."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl Focusable for TableStory {
    fn focus_handle(&self, cx: &App) -> gpui::FocusHandle {
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
                .placeholder("Enter number of Stocks to display")
                .validate(|s, _| s.parse::<usize>().is_ok());
            input.set_value("15", window, cx);
            input
        });

        let delegate = StockTableDelegate::new(15);
        let table = cx.new(|cx| Table::new(delegate, window, cx));

        cx.subscribe_in(&table, window, Self::on_table_event)
            .detach();
        cx.subscribe_in(&num_stocks_input, window, Self::on_num_stocks_input_change)
            .detach();

        // Spawn a background to random refresh the list
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(Duration::from_millis(33)).await;

                this.update(cx, |this, cx| {
                    if !this.refresh_data {
                        return;
                    }

                    this.table.update(cx, |table, _| {
                        table.delegate_mut().stocks.iter_mut().enumerate().for_each(
                            |(i, stock)| {
                                let n = (3..10).fake::<usize>();
                                // update 30% of the stocks
                                if i.is_multiple_of(n) {
                                    stock.random_update();
                                }
                            },
                        );
                    });
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();

        Self {
            table,
            num_stocks_input,
            stripe: false,
            refresh_data: false,
            size: Size::default(),
        }
    }

    // Event handler for changes in the number input field
    fn on_num_stocks_input_change(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            // Update when the user presses Enter or the input loses focus
            InputEvent::PressEnter { .. } | InputEvent::Blur => {
                let text = self.num_stocks_input.read(cx).value().to_string();
                if let Ok(total_count) = text.parse::<usize>() {
                    if total_count == self.table.read(cx).delegate().stocks.len() {
                        return;
                    }

                    self.table.update(cx, |table, _| {
                        table.delegate_mut().update_stocks(total_count);
                    });
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    fn toggle_loop_selection(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.loop_selection = *checked;
            cx.notify();
        });
    }

    fn toggle_col_resize(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.col_resizable = *checked;
            cx.notify();
        });
    }

    fn toggle_col_order(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.col_movable = *checked;
            cx.notify();
        });
    }

    fn toggle_col_sort(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.sortable = *checked;
            cx.notify();
        });
    }

    fn toggle_col_fixed(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.col_fixed = *checked;
            cx.notify();
        });
    }

    fn toggle_col_selection(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.col_selectable = *checked;
            cx.notify();
        });
    }

    fn toggle_stripe(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.stripe = *checked;
        let stripe = self.stripe;
        self.table.update(cx, |table, cx| {
            table.set_stripe(stripe, cx);
            cx.notify();
        });
    }

    fn on_change_size(&mut self, a: &ChangeSize, _: &mut Window, cx: &mut Context<Self>) {
        self.size = a.0;
        self.table.update(cx, |table, cx| {
            table.set_size(a.0, cx);
            table.delegate_mut().size = a.0;
        });
    }

    fn toggle_refresh_data(&mut self, checked: &bool, _: &mut Window, cx: &mut Context<Self>) {
        self.refresh_data = *checked;
        cx.notify();
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
                    .items_center()
                    .gap_3()
                    .flex_wrap()
                    .child(
                        Checkbox::new("loop-selection")
                            .label("Loop Selection")
                            .selected(table.loop_selection)
                            .on_click(cx.listener(Self::toggle_loop_selection)),
                    )
                    .child(
                        Checkbox::new("col-resize")
                            .label("Column Resize")
                            .selected(table.col_resizable)
                            .on_click(cx.listener(Self::toggle_col_resize)),
                    )
                    .child(
                        Checkbox::new("col-order")
                            .label("Column Order")
                            .selected(table.col_movable)
                            .on_click(cx.listener(Self::toggle_col_order)),
                    )
                    .child(
                        Checkbox::new("col-sort")
                            .label("Sortable")
                            .selected(table.sortable)
                            .on_click(cx.listener(Self::toggle_col_sort)),
                    )
                    .child(
                        Checkbox::new("col-selection")
                            .label("Column Selectable")
                            .selected(table.col_selectable)
                            .on_click(cx.listener(Self::toggle_col_selection)),
                    )
                    .child(
                        Checkbox::new("fixed")
                            .label("Column Fixed")
                            .selected(table.col_fixed)
                            .on_click(cx.listener(Self::toggle_col_fixed)),
                    )
                    .child(
                        Checkbox::new("stripe")
                            .label("Stripe")
                            .selected(self.stripe)
                            .on_click(cx.listener(Self::toggle_stripe)),
                    )
                    .child(
                        Checkbox::new("loading")
                            .label("Loading")
                            .checked(self.table.read(cx).delegate().full_loading)
                            .on_click(cx.listener(|this, check: &bool, _, cx| {
                                this.table.update(cx, |this, cx| {
                                    this.delegate_mut().full_loading = *check;
                                    cx.notify();
                                })
                            })),
                    )
                    .child(
                        Checkbox::new("refresh-data")
                            .label("Refresh Data")
                            .selected(self.refresh_data)
                            .on_click(cx.listener(Self::toggle_refresh_data)),
                    ),
            )
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
