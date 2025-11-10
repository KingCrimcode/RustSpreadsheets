use std::rc::Rc;

use dioxus::{core::spawn_forever, prelude::*};
use petgraph::{
    algo,
    visit::{self, Dfs},
};
use tracing::info;

use crate::{
    engine::parser::{self, FormulaError},
    model::grid::{column_index_to_letter, cell_address_to_coords, Cell, Coords, Grid},
};

pub fn update_cell_display(mut grid: Signal<Grid>, coords: Coords) {
    let mut cycle: bool = false;
    let dependants: Vec<_>;
    info!("Called for {:?}", coords);
    {
        info!("Entered brackets");
        let mut grid_write = grid.write();
        let Some(content) = grid_write.cells_map.get(&coords).map(|c| c.content.clone()) else {
            return;
        };
        grid_write.remove_cell_dependencies(coords);
        let display_value: String;
        if content.starts_with('=') {
            if let Some(target_coords) = cell_address_to_coords(content.split_at(1).1) {
                display_value = match grid_write.get_cell_value_by_address(content.split_at(1).1) {
                    Ok(value) => value.to_string(),
                    Err(err) => err.to_string(),
                };
                grid_write
                    .cells_dep_graph
                    .add_edge(target_coords, coords, ());
            } else {
                let cell_ref_resolver =
                    |ref_str: &str| grid_write.get_cell_value_by_address(ref_str);
                display_value = match parser::calculate(&content, &cell_ref_resolver) {
                    Ok((val, deps)) => {
                        deps.into_iter().for_each(|dep| {
                            if let Some(dep_coords) = cell_address_to_coords(&dep) {
                                grid_write.cells_dep_graph.add_edge(dep_coords, coords, ());
                            }
                        });
                        val.to_string()
                    }
                    Err(e) => e.to_string(),
                };
            }
            if is_node_in_cycle(&grid_write.cells_dep_graph, coords) {
                cycle = true;
            }
        } else {
            display_value = content;
        }
        grid_write.cells_map.get_mut(&coords).unwrap().display_value = match cycle {
            false => display_value,
            true => FormulaError::CircularReference.to_string(),
        };
        dependants = grid_write.get_cell_dependants(coords);
    }
    if !cycle {
        dependants.into_iter().for_each(|dependant| {
            update_cell_display(grid, dependant);
        });
    } else {
        let nodes = get_connected_nodes(&grid.read().cells_dep_graph, coords);
        nodes.iter().for_each(|node| {
            grid.write().cells_map.get_mut(node).unwrap().display_value =
                FormulaError::CircularReference.to_string();
        });
        info!("Connected nodes: {:?}", nodes);
    }
}

fn is_node_in_cycle<G>(graph: G, node: G::NodeId) -> bool
where
    G: visit::IntoNeighbors + visit::Visitable,
{
    let mut space = algo::DfsSpace::new(&graph);
    for neighbour in graph.neighbors(node) {
        if algo::has_path_connecting(graph, neighbour, node, Some(&mut space)) {
            return true;
        }
    }
    false
}

fn get_connected_nodes<G>(graph: G, node: G::NodeId) -> <G as petgraph::visit::Visitable>::Map
where
    G: visit::IntoNeighbors + visit::Visitable,
{
    let mut dfs = Dfs::new(&graph, node);
    while dfs.next(&graph).is_some() {}
    dfs.discovered
}

static GRID_CSS: Asset = asset!("/assets/grid.css");

#[component]
pub fn GridDisplay(
    grid: Signal<Grid>,
    scroll_container: Signal<Option<Rc<MountedData>>>,
) -> Element {
    rsx! {
        document::Stylesheet { href: GRID_CSS }
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
                        grid.write().current_cell_down_one();
                    }
                    Key::ArrowUp => {
                        grid.write().current_cell_up_one();
                    }
                    Key::ArrowLeft => {
                        grid.write().current_cell_left_one();
                    }
                    Key::ArrowRight => {
                        grid.write().current_cell_right_one();
                    }
                    Key::Enter => {
                        if evt.modifiers().shift() {
                            grid.write().current_cell_up_one();
                        } else {
                            grid.write().current_cell_down_one();
                        }
                    }
                    Key::Tab => {
                        if evt.modifiers().shift() {
                            grid.write().current_cell_left_one();
                        } else {
                            grid.write().current_cell_right_one();
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
                let grid_read = grid.read();
                let grid_template_columns = format!(
                    "{}px {}",
                    grid_read.base_header_column_width,
                    grid_read
                        .column_widths
                        .iter()
                        .map(|width| format!("{}px", width))
                        .collect::<Vec<_>>()
                        .join(" ")
                    );
                let grid_template_rows = format!(
                    "{}px {}",
                    grid_read.base_header_row_height,
                    grid_read
                        .row_heights
                        .iter()
                        .map(|height| format!("{}px", height))
                        .collect::<Vec<_>>()
                        .join(" ")
                    );
                rsx! {
                    div {
                        class: "grid",
                        style: "grid-template-columns: {grid_template_columns}; grid-template-rows: {grid_template_rows};",

                        CornerCell { grid }
                        HeaderRow { grid }
                        HeaderColumn { grid }
                        GridCells { grid, scroll_container }
                    }
                }
            }
        }
    }
}

#[component]
fn CornerCell(grid: Signal<Grid>) -> Element {
    rsx! {
        div {
            class: "corner-cell",
            style: "grid-row: 1; grid-column: 1;",
        }
    }
}

#[component]
fn HeaderRow(grid: Signal<Grid>) -> Element {
    rsx! {
        for col in 0..grid.read().column_widths.len() as i32 {
            div {
                class: "column-header header-cell",
                style: "grid-row: 1; grid-column: {col + 2};",
                "{column_index_to_letter(col)}"
            }
        }
    }
}

#[component]
fn HeaderColumn(grid: Signal<Grid>) -> Element {
    rsx! {
        for row in 0..grid.read().row_heights.len() {
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
        for row in 0..grid.read().row_heights.len() {
            for col in 0..grid.read().column_widths.len() {
                {
                    let grid_read = grid.read();
                    let coords = Coords { row: row as i32, column: col as i32 };
                    let cell = grid_read.cells_map.get(&coords);

                    let display_value = cell.map(|c| c.display_value.as_str()).unwrap_or_default();
                    let sci_noatation = match display_value.parse::<f64>() {
                        Ok(val) => format!("{:.2e}", val),
                        Err(_) => display_value.to_string(),
                    };
                    // Number of characters that can fit in the cell
                    // 5 - border + padding size
                    // 7 - font size (idk, works)
                    // NOTE: change this after implementing changable font & border size
                    let char_space = (grid_read.column_widths[col] - 5 * 2) / 7;

                    let is_selected = grid_read.current_cell == coords;
                    let top_is_selected = grid_read.current_cell == Coords { row: row as i32 - 1 , column: col as i32 };
                    let left_is_selected = grid_read.current_cell == Coords { row: row as i32, column: col as i32 - 1 };

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
                                grid.write().current_cell = Coords { row: row as i32, column: col as i32};
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
    row: usize,
    col: usize,
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
                if grid.write().is_editing_cell {
                    update_cell_display(grid, coords);
                }
                grid.write().is_editing_cell = false;
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
                                grid.write().current_cell_up_one();
                            }
                            else {
                                grid.write().current_cell_down_one();
                            }
                        } else if evt.modifiers().shift() {
                            grid.write().current_cell_left_one();
                        }
                        else {
                            grid.write().current_cell_right_one();
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
