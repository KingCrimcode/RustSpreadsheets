use std::rc::Rc;
use wasm_bindgen::JsCast;

use dioxus::{core::spawn_forever, prelude::*};

use crate::{
    components::grid::update_cell_display,
    model::grid::{cell_address_to_coords, Cell, Coords, Grid},
};

static HEADER_CSS: Asset = asset!("/assets/header.css");

#[component]
pub fn Header(grid: Signal<Grid>, scroll_container: Signal<Option<Rc<MountedData>>>) -> Element {
    rsx! {
        document::Stylesheet { href: HEADER_CSS }
        div {
            class: "header",

            FileToolbar { grid },
            FormattingToolbar {},
            FormulaBar { grid, scroll_container }
        }
    }
}

#[component]
fn FileToolbar(grid: Signal<Grid>) -> Element {
    rsx! {
        div {
            class: "file-toolbar",

            button {
                "tooltip-text": "Export to CSV",
                onclick: move |_| {
                    let csv = export_to_csv(grid);

                    let array = js_sys::Array::new();
                    array.push(&wasm_bindgen::JsValue::from_str(&csv));
                    let blob = web_sys::Blob::new_with_str_sequence(&array).unwrap();
                    let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();

                    let document = web_sys::window().unwrap().document().unwrap();
                    let anchor: web_sys::HtmlAnchorElement = document.create_element("a").unwrap().dyn_into().unwrap();

                    anchor.set_href(&url);
                    anchor.set_download("export.csv");
                    anchor.click();

                    web_sys::Url::revoke_object_url(&url).unwrap();
                },
                lucide_dioxus::Save { size: 22 }
            }

            input {
                r#type: "file",
                accept: ".csv",
                id: "csv-import",
                style: "display: none;",
                onchange: move |evt| {
                    spawn(async move {
                        if let Some(file) = evt.files().first() {
                            match file.read_string().await {
                                Ok(csv_text) => {
                                    import_csv(grid, &csv_text);
                                }
                                Err(e) => {
                                    error!("{e:?}");
                                }
                            }
                        }
                    });
                }
            }
            button {
                "tooltip-text": "Import CSV",
                onclick: move |_| {
                    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                        if let Some(element) = document.get_element_by_id("csv-import") {
                            if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
                                input.click();
                            }
                        }
                    }
                },
                lucide_dioxus::FolderOpen { size: 22 }
            }
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

fn export_to_csv(grid: Signal<Grid>) -> String {
    let row_count = grid
        .read()
        .cells_map
        .keys()
        .map(|c| c.row)
        .max()
        .unwrap_or(0);
    let col_count = grid
        .read()
        .cells_map
        .keys()
        .map(|c| c.column)
        .max()
        .unwrap_or(0);

    let mut lines = Vec::new();
    for row in 0..=row_count {
        let mut cells = Vec::new();
        for col in 0..=col_count {
            let coords = Coords { row, column: col };
            let content = grid
                .read()
                .cells_map
                .get(&coords)
                .map(|c| c.content.clone())
                .unwrap_or_default();
            cells.push(content);
        }
        lines.push(cells.join(","));
    }
    lines.join("\n")
}

fn import_csv(mut grid: Signal<Grid>, csv_text: &str) {
    let mut coords_list: Vec<Coords> = Vec::new();
    csv_text.lines().enumerate().for_each(|(row, line)| {
        line.split(',').enumerate().for_each(|(col, content)| {
            let coords = Coords {
                row: row as i32,
                column: col as i32,
            };
            coords_list.push(coords);
            grid.write()
                .cells_map
                .entry(coords)
                .or_insert(Cell::new())
                .content = content.to_string();
        });
    });
    coords_list.into_iter().for_each(|coords| {
        update_cell_display(grid, coords);
    });
}
