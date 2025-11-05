use std::{collections::HashMap, rc::Rc};

use dioxus::{document::Stylesheet, prelude::*};

use crate::engine::parser;

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

fn column_letter_to_index(column: &str) -> i64 {
    let mut result = 0;
    for c in column.chars().map(|c| c.to_ascii_uppercase()) {
        result = result * 26 + (c as i64 - 'A' as i64 + 1);
    }
    result
}

fn cell_address_to_coords(address: &str) -> Option<Coords> {
    let col_end = address.find(|c: char| c.is_numeric())?;
    let col = column_letter_to_index(&address[..col_end]);
    let row = &address[col_end..].parse::<i64>().ok()?;
    Some(Coords {
        row: row - 1,
        column: col - 1,
    })
}

pub fn update_cell_display(mut grid: Signal<Grid>, coords: Coords) {
    let mut grid_write = grid.write();
    let Some(content) = grid_write.cells_map.get(&coords).map(|c| c.content.clone()) else {
        return;
    };
    if content.starts_with('=') {
        let cell_ref_resolver = |ref_str: &str| grid_write.get_cell_value_by_address(ref_str);
        let display_value = match parser::calculate(&content, &cell_ref_resolver) {
            Ok(val) => val.to_string(),
            Err(e) => e.to_string(),
        };
        grid_write.cells_map.get_mut(&coords).unwrap().display_value = display_value;
    } else {
        grid_write.cells_map.get_mut(&coords).unwrap().display_value = content;
    }
}

#[component]
pub fn GridDisplay(
    grid: Signal<Grid>,
    scroll_container: Signal<Option<Rc<MountedData>>>,
) -> Element {
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
                        if evt.modifiers().shift() {
                            grid.write().current_cell.up_one();
                        } else {
                            grid.write().current_cell.down_one();
                        }
                    }
                    Key::Tab => {
                        if evt.modifiers().shift() {
                            grid.write().current_cell.left_one();
                        } else {
                            grid.write().current_cell.right_one();
                        }
                    }
                    Key::Character(c) => if c.len() == 1 {
                        let previous_value = grid.write().get_current_cell_content().clone();
                        grid.write().previous_content = previous_value;
                        grid.write().get_mut_current_cell().content = c;
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
                    let grid_read = grid.read();
                    let coords = Coords { row: row as i64, column: col as i64 };
                    let cell = grid_read.cells_map.get(&coords);

                    let display_value = cell.map(|c| c.display_value.as_str()).unwrap_or_default();
                    let sci_noatation = match display_value.parse::<f64>() {
                        Ok(val) => format!("{:.2e}", val),
                        Err(_) => display_value.to_string(),
                    };
                    // Number of characters that can fit in the cell
                    // 5 - border + padding size
                    // 7 - font size (11pt to px)
                    // NOTE: change this after implementing changable font & border size
                    let char_space = (grid_read.column_widths[col as usize] - 5 * 2) / 7;

                    let is_selected = grid_read.current_cell == coords;
                    let top_is_selected = grid_read.current_cell == Coords { row: (row - 1) as i64, column: col as i64 };
                    let left_is_selected = grid_read.current_cell == Coords { row: row as i64, column: (col - 1) as i64 };

                    let cell_class =
                        if is_selected { "cell cell-selected" }
                        else if top_is_selected { "cell cell-selected-up " }
                        else if left_is_selected { "cell cell-selected-left" }
                        else { "cell" };

                    let is_editing = grid_read.is_editing_cell && is_selected;

                    rsx! {
                        div {
                            class: "{cell_class}",
                            style: "grid-row: {row + 2}; grid-column: {col + 2};",
                            onclick: move |_| {
                                grid.write().current_cell = Coords { row: row as i64, column: col as i64};
                            },
                            ondoubleclick: move |_| {
                                let previous_value = grid.write().get_current_cell_content().clone();
                                grid.write().previous_content = previous_value;
                                grid.write().is_editing_cell = true;
                            },
                            if !is_editing {
                                if display_value.is_empty() || display_value.len() as i32 <= char_space {
                                    "{display_value}"
                                } else if sci_noatation.len() as i32 <= char_space {
                                    "{sci_noatation}"
                                } else {
                                    "###"
                                }
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
            },
            onblur: move |_| {
                grid.write().is_editing_cell = false;
                update_cell_display(grid, coords);
            },
            onkeydown: move |evt| {
                evt.stop_propagation();
                match evt.key() {
                    Key::Enter | Key::Tab => {
                        evt.prevent_default();
                        grid.write().is_editing_cell = false;
                        update_cell_display(grid, coords);

                        if evt.key() == Key::Enter {
                            if evt.modifiers().shift() {
                                grid.write().current_cell.up_one();
                            }
                            else {
                                grid.write().current_cell.down_one();
                            }
                        } else if evt.modifiers().shift() {
                            grid.write().current_cell.left_one();
                        }
                        else {
                            grid.write().current_cell.right_one();
                        }

                        if let Some(container) = scroll_container() {
                            spawn_forever(async move {
                                let _ = container.set_focus(true).await;
                            });
                        }
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        let previous_content = grid.write().previous_content.clone();
                        grid.write().cells_map.entry(coords).or_insert(Cell::new()).content = previous_content; 
                        grid.write().is_editing_cell = false;
                        update_cell_display(grid, coords);

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
    pub previous_content: String,
}

impl Grid {
    pub fn new() -> Self {
        Grid {
            cells_map: HashMap::new(),
            current_cell: Coords { row: 0, column: 0 },
            is_editing_cell: false,
            column_widths: vec![HEADER_COLUMN_WIDTH; 1000],
            row_heights: vec![HEADER_ROW_HEIGHT; 1000],
            previous_content: String::new(),
        }
    }
    pub fn get_mut_current_cell(&mut self) -> &mut Cell {
        self.cells_map.entry(self.current_cell).or_insert(Cell::new())
    }
    pub fn get_current_cell_address(&self) -> String {
        format!(
            "{}{}",
            column_index_to_letter(self.current_cell.column as i32),
            self.current_cell.row + 1
        )
    }
    pub fn get_current_cell_content(&self) -> String {
        self.cells_map
            .get(&self.current_cell)
            .map(|c| c.content.clone())
            .unwrap_or_default()
    }
    pub fn get_cell_value_by_address(&self, address: &str) -> Option<f64> {
        let coords = cell_address_to_coords(address)?;
        self.cells_map
            .get(&coords)
            .and_then(|c| c.display_value.parse().ok())
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

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
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
