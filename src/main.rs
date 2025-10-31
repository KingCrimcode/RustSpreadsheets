use dioxus::document::Stylesheet;
use dioxus::prelude::*;

use crate::components::{
    grid::{Grid, GridDisplay},
    header::Header,
};

mod components;
mod engine;

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
    let grid = use_signal(Grid::new);
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
