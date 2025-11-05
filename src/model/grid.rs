use std::collections::HashMap;

pub fn column_index_to_letter(column: i32) -> String {
    let mut result = String::new();
    let mut column = column;
    while column >= 0 {
        result.insert(0, (b'A' + (column % 26) as u8) as char);
        column = column / 26 - 1;
    }
    result
}

fn column_letter_to_index(column: &str) -> i32 {
    let mut result = 0;
    for c in column.chars().map(|c| c.to_ascii_uppercase()) {
        result = result * 26 + (c as i32 - 'A' as i32 + 1);
    }
    result
}

fn cell_address_to_coords(address: &str) -> Option<Coords> {
    let col_end = address.find(|c: char| c.is_numeric())?;
    let col = column_letter_to_index(&address[..col_end]);
    let row = &address[col_end..].parse::<i32>().ok()?;
    Some(Coords {
        row: row - 1,
        column: col - 1,
    })
}

pub struct Grid {
    pub cells_map: HashMap<Coords, Cell>,

    pub current_cell: Coords,
    pub previous_content: String,
    pub is_editing_cell: bool,

    pub base_header_column_width: i32,
    pub base_header_row_height: i32,
    pub column_widths: Vec<i32>,
    pub row_heights: Vec<i32>,
}

impl Grid {
    pub fn new(base_header_column_width: i32, base_header_row_height: i32, column_count: usize, row_count: usize) -> Self {
        Grid {
            cells_map: HashMap::new(),

            current_cell: Coords { row: 0, column: 0 },
            previous_content: String::new(),
            is_editing_cell: false,

            base_header_column_width,
            base_header_row_height,
            column_widths: vec![base_header_column_width; column_count],
            row_heights: vec![base_header_row_height; row_count],
        }
    }

    pub fn get_cell_value_by_address(&self, address: &str) -> Option<f64> {
        let coords = cell_address_to_coords(address)?;
        self.cells_map
            .get(&coords)
            .and_then(|c| c.display_value.parse().ok())
    }

    pub fn get_mut_current_cell(&mut self) -> &mut Cell {
        self.cells_map.entry(self.current_cell).or_insert(Cell::new())
    }
    pub fn get_current_cell_address(&self) -> String {
        format!(
            "{}{}",
            column_index_to_letter(self.current_cell.column),
            self.current_cell.row + 1
        )
    }
    pub fn get_current_cell_content(&self) -> String {
        self.cells_map
            .get(&self.current_cell)
            .map(|c| c.content.clone())
            .unwrap_or_default()
    }

    pub fn current_cell_up_one(&mut self) {
        if self.current_cell.row > 0 {
            self.current_cell.row -= 1;
        }
    }
    pub fn current_cell_down_one(&mut self) {
        if self.current_cell.row < (self.row_heights.len() - 1) as i32 {
            self.current_cell.row += 1;
        }
    }
    pub fn current_cell_left_one(&mut self) {
        if self.current_cell.column > 0 {
            self.current_cell.column -= 1;
        }
    }
    pub fn current_cell_right_one(&mut self) {
        if self.current_cell.column < (self.column_widths.len() - 1) as i32 {
            self.current_cell.column += 1;
        }
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
    pub row: i32,
    pub column: i32,
}
