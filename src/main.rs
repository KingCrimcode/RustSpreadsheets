use dioxus::document::Stylesheet;
use dioxus::prelude::*;

use crate::{components::{
    grid::GridDisplay,
    header::Header,
}, model::grid::Grid};

mod components;
mod engine;
mod model;

const HEADER_COLUMN_WIDTH: i32 = 90;
const HEADER_ROW_HEIGHT: i32 = 25;
const CELL_COLUMNS: usize = 26;
const CELL_ROWS: usize = 100;


fn main() {
    tracing_wasm::set_as_global_default_with_config(
        tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(tracing::Level::INFO)
            .build(),
    );
    dioxus::launch(app);
}

#[component]
fn app() -> Element {
    let grid = use_signal(|| Grid::new(HEADER_COLUMN_WIDTH, HEADER_ROW_HEIGHT, CELL_COLUMNS, CELL_ROWS));
    let scroll_container = use_signal(|| None);

    rsx! {
        Stylesheet { href: asset!("/assets/colorscheme.css") }
        Stylesheet { href: asset!("/assets/main.css") }
        body {
            Header { grid, scroll_container }
            GridDisplay { grid, scroll_container }
        }
    }
}
