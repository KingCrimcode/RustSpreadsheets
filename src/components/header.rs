use std::rc::Rc;

use dioxus::{core::spawn_forever, document::Stylesheet, prelude::*};

use crate::{
    components::grid::update_cell_display,
    model::grid::{cell_address_to_coords, Cell, Grid},
};

#[component]
pub fn Header(grid: Signal<Grid>, scroll_container: Signal<Option<Rc<MountedData>>>) -> Element {
    rsx! {
        Stylesheet { href: asset!("/assets/header.css") }
        div {
            class: "header",

            FileToolbar {},
            FormattingToolbar {},
            FormulaBar { grid, scroll_container }
        }
    }
}

#[component]
fn FileToolbar() -> Element {
    rsx! {
        div {
            class: "file-toolbar",
        }
    }
}

#[component]
fn FormattingToolbar() -> Element {
    rsx! {
        div {
            class: "formatting-toolbar",
        }
    }
}

#[component]
fn FormulaBar(grid: Signal<Grid>, scroll_container: Signal<Option<Rc<MountedData>>>) -> Element {
    rsx! {
        div {
            class: "formula-bar",

            CellAddressInput { grid, scroll_container },
            div { class: "formula-bar-separator header-input", "│" },
            div { class: "formula-bar-fx header-input", "fx" },
            FormulaInput { grid, scroll_container }
        }
    }
}

#[component]
fn CellAddressInput(
    grid: Signal<Grid>,
    scroll_container: Signal<Option<Rc<MountedData>>>,
) -> Element {
    let mut previous_address = use_signal(|| grid.read().get_current_cell_address().clone());
    let mut value = use_signal(String::new);

    rsx! {
        input {
            class: "cell-address-input header-input",
            value: "{grid.read().get_current_cell_address()}",
            onfocus: move |_| {
                previous_address.set(grid.read().get_current_cell_address());
            },
            oninput: move |evt| {
                value.set(evt.value());
            },
            onkeydown: move |evt| {
                // evt.stop_propagation();
                match evt.key() {
                    Key::Enter => {
                        if let Some(new_coords) = cell_address_to_coords(&value.read()) {
                            grid.write().current_cell = new_coords;
                        } else {
                            grid.write().current_cell = cell_address_to_coords(&previous_address.read()).unwrap();
                        }
                        if let Some(container) = scroll_container() {
                            spawn_forever(async move {
                                let _ = container.set_focus(true).await;
                            });
                        }
                    }
                    Key::Escape => {
                        grid.write().current_cell = cell_address_to_coords(&previous_address.read()).unwrap();
                        if let Some(container) = scroll_container() {
                            spawn_forever(async move {
                                let _ = container.set_focus(true).await;
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[component]
fn FormulaInput(grid: Signal<Grid>, scroll_container: Signal<Option<Rc<MountedData>>>) -> Element {
    let mut previous_value = use_signal(String::new);

    rsx! {
        input {
            class: "formula-input header-input",
            value: "{grid.read().get_current_cell_content()}",
            onfocus: move |_| {
                previous_value.set(grid.read().get_current_cell_content());
            },
            oninput: move |evt| {
                let coords = grid.read().current_cell;
                let mut grid_write = grid.write();
                let cell = grid_write.cells_map.entry(coords).or_insert(Cell::new());
                cell.content = evt.value();
                cell.display_value = evt.value();
            },
            onkeydown: move |evt| {
                // evt.stop_propagation();
                match evt.key() {
                    Key::Enter => {
                        evt.prevent_default();
                        let coords = grid.read().current_cell;
                        update_cell_display(grid, coords);
                        if let Some(container) = scroll_container() {
                            spawn_forever(async move {
                                let _ = container.set_focus(true).await;
                            });
                        }
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        let coords = grid.read().current_cell;
                        let previous_content = grid.write().previous_content.clone();
                        grid.write().cells_map.entry(coords).or_insert(Cell::new()).content = previous_content;
                        update_cell_display(grid, coords);

                        if let Some(container) = scroll_container() {
                            spawn_forever(async move {
                                let _ = container.set_focus(true).await;
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
