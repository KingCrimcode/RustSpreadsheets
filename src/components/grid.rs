use std::{collections::HashMap, rc::Rc};

use dioxus::{document::Stylesheet, prelude::*};

const HEADER_ROW_HEIGHT: i32 = 25;
const HEADER_COLUMN_WIDTH: i32 = 90;
const CELL_ROWS: i32 = 100;
const CELL_COLUMNS: i32 = 26;

fn column_index_to_letter(column: i32) -> String {
    let mut result = String::new();
    let mut column = column;
    while column >= 0 {
        result.insert(0, (b'A' + (column % 26) as u8) as char);
        column = column / 26 - 1;
    }
    result
}

#[component]
pub fn GridDisplay(grid: Signal<Grid>, scroll_container: Signal<Option<Rc<MountedData>>>) -> Element {
    rsx! {
        Stylesheet { href: asset!("/assets/grid.css") }
        div {
            class: "scroll-container",
            tabindex: "0",

            onmounted: move |elem| async move {
                scroll_container.set(Some(elem.data()));
                let _ = elem.data().set_focus(true).await;
            },

            onkeydown: move |evt| {
                evt.prevent_default();
                match evt.key() {
                    Key::ArrowDown => {
                        grid.write().current_cell.down_one();
                    }
                    Key::ArrowUp => {
                        grid.write().current_cell.up_one();
                    }
                    Key::ArrowLeft => {
                        grid.write().current_cell.left_one();
                    }
                    Key::ArrowRight => {
                        grid.write().current_cell.right_one();
                    }
                    Key::Enter => {
                        grid.write().is_editing_cell = true;
                    }
                    _ => {}
                }
            },

            {
                let grid_ref = grid.read();
                let grid_template_columns = format!(
                    "{}px {}",
                    HEADER_COLUMN_WIDTH,
                    grid_ref
                        .column_widths
                        .iter()
                        .take(CELL_COLUMNS as usize)
                        .map(|width| format!("{}px", width))
                        .collect::<Vec<_>>()
                        .join(" ")
                    );
                let grid_template_rows = format!(
                    "{}px {}",
                    HEADER_ROW_HEIGHT,
                    grid_ref
                        .row_heights
                        .iter()
                        .take(CELL_ROWS as usize)
                        .map(|height| format!("{}px", height))
                        .collect::<Vec<_>>()
                        .join(" ")
                    );
                rsx! {
                    div {
                        class: "grid",
                        style: "grid-template-columns: {grid_template_columns}; grid-template-rows: {grid_template_rows};",

                        CornerCell {  }
                        HeaderRow {  }
                        HeaderColumn {  }
                        GridCells { grid, scroll_container }
                    }
                }
            }
        }
    }
}

#[component]
fn CornerCell() -> Element {
    rsx! {
        div {
            class: "corner-cell",
            style: "grid-row: 1; grid-column: 1;",
        }
    }
}

#[component]
fn HeaderRow() -> Element {
    rsx! {
        for col in 0..CELL_COLUMNS {
            div {
                class: "column-header header-cell",
                style: "grid-row: 1; grid-column: {col + 2};",
                "{column_index_to_letter(col)}"
            }
        }
    }
}

#[component]
fn HeaderColumn() -> Element {
    rsx! {
        for row in 0..CELL_ROWS {
            div {
                class: "row-header header-cell",
                style: "grid-row: {row + 2}; grid-column: 1;",
                "{row + 1}"
            }
        }
    }
}

#[component]
fn GridCells(grid: Signal<Grid>, scroll_container: Signal<Option<Rc<MountedData>>>) -> Element {
    rsx! {
        for row in 0..CELL_ROWS {
            for col in 0..CELL_COLUMNS {
                {
                    let grid_ref = grid.read();
                    let coords = Coords { row: row as i64, column: col as i64 };
                    let cell = grid_ref.cells_map.get(&coords);
                    let display = cell.map(|c| c.display_value.as_str()).unwrap_or_default();

                    let is_selected = grid_ref.current_cell == coords;
                    let top_is_selected = grid_ref.current_cell == Coords { row: (row - 1) as i64, column: col as i64 };
                    let left_is_selected = grid_ref.current_cell == Coords { row: row as i64, column: (col - 1) as i64 };

                    let cell_class =
                        if is_selected { "cell cell-selected" }
                        else if top_is_selected { "cell cell-selected-up " }
                        else if left_is_selected { "cell cell-selected-left" }
                        else { "cell" };

                    let is_editing = grid_ref.is_editing_cell && is_selected;

                    rsx! {
                        div {
                            class: "{cell_class}",
                            style: "grid-row: {row + 2}; grid-column: {col + 2};",
                            onclick: move |_| {
                                grid.write().current_cell = Coords { row: row as i64, column: col as i64};
                                grid.write().is_editing_cell = false;
                            },
                            ondoubleclick: move |_| {
                                grid.write().is_editing_cell = true;
                            },
                            if !is_editing {
                                "{display}"
                            }
                        }
                        if is_editing {
                            InputCell { grid, scroll_container, coords, row, col }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn InputCell(
    grid: Signal<Grid>,
    scroll_container: Signal<Option<Rc<MountedData>>>,
    coords: Coords,
    row: i32,
    col: i32,
) -> Element {
    let previous_value = use_signal(|| grid.read().get_current_cell_content().clone());

    rsx! {
        input {
            class: "input-cell",
            style: "grid-row: {row + 2}; grid-column: {col + 2};",
            value: "{grid.read().get_current_cell_content()}",
            onmounted: move |elem| async move {
                let _ = elem.data().set_focus(true).await;
            },
            oninput: move |evt| {
                let mut grid_write = grid.write();
                let cell = grid_write.cells_map.entry(coords).or_insert(Cell::new());
                cell.content = evt.value();
                cell.display_value = evt.value();
            },
            onblur: move |_| {
                grid.write().is_editing_cell = false;
            },
            onkeydown: move |evt| {
                evt.stop_propagation();
                match evt.key() {
                    Key::Enter => {
                        grid.write().is_editing_cell = false;

                        if let Some(container) = scroll_container() {
                            spawn_forever(async move {
                                let _ = container.set_focus(true).await;
                            });
                        }
                    }
                    Key::Escape => {
                        let mut grid_write = grid.write();
                        let cell = grid_write.cells_map.entry(coords).or_insert(Cell::new());
                        cell.content = previous_value();
                        cell.display_value = previous_value();

                        grid_write.is_editing_cell = false;

                        if let Some(container) = scroll_container() {
                            spawn_forever(async move {
                                let _ = container.set_focus(true).await;
                            });
                        }
                    }
                    _ => { }
                }
            }
        }
    }
}

pub struct Grid {
    pub cells_map: HashMap<Coords, Cell>,
    pub current_cell: Coords,
    is_editing_cell: bool,
    column_widths: Vec<i32>,
    row_heights: Vec<i32>,
}

impl Grid {
    pub fn new() -> Self {
        Grid {
            cells_map: HashMap::new(),
            current_cell: Coords { row: 0, column: 0 },
            is_editing_cell: false,
            column_widths: vec![HEADER_COLUMN_WIDTH; 1000],
            row_heights: vec![HEADER_ROW_HEIGHT; 1000],
        }
    }
    pub fn get_current_cell_address(&self) -> String {
        format!("{}{}", column_index_to_letter(self.current_cell.column as i32), self.current_cell.row + 1)
    }
    pub fn get_current_cell_content(&self) -> String {
        self.cells_map.get(&self.current_cell).map(|c| c.content.clone()).unwrap_or_default()
    }
}

pub struct Cell {
    pub content: String,
    pub display_value: String,
}

impl Cell {
    pub fn new() -> Self {
        Cell {
            content: String::new(),
            display_value: String::new(),
        }
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub struct Coords {
    row: i64,
    column: i64,
}

impl Coords {
    fn up_one(&mut self) {
        if self.row > 0 {
            self.row -= 1;
        }
    }

    fn down_one(&mut self) {
        if self.row < (CELL_ROWS - 1) as i64 {
            self.row += 1;
        }
    }

    fn left_one(&mut self) {
        if self.column > 0 {
            self.column -= 1;
        }
    }

    fn right_one(&mut self) {
        if self.column < (CELL_COLUMNS - 1) as i64 {
            self.column += 1;
        }
    }
}
