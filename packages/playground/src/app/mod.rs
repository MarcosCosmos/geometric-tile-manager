use eframe::emath::{Rect, Vec2};
use eframe::epaint::{CornerRadius, Stroke};
use egui::epaint::RectShape;
use egui::{Color32, Rangef, Sense, StrokeKind};
use geotm_core::command::add_surface::AddSurface;
use geotm_core::command::add_tile::AddTile;
use geotm_core::command::{CommandExecutor, RawCommandExecutor};
use geotm_core::context::QueryContext;
use geotm_core::error::KeyedError;
use geotm_core::geometry::Axis::{Horizontal, Vertical};
use geotm_core::geometry::Sign::{Negative, Positive};
use geotm_core::geometry::{AreaBounds, AxisBounds, EAST, NORTH, SOUTH, WEST};
use geotm_core::graph::Graph;
use geotm_core::iter::IterativeQueryNavigation;
use geotm_core::line::Line;
use geotm_core::neighbours::NeighbourTiles;
use geotm_core::node;
use geotm_core::node::NodeQuery;
use geotm_core::query::{
    ElementInformation, ElementInformation1D, ElementInformation2D, ElementKind, Query,
};
use geotm_util::focus::FocusState;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use snafu::Report;

pub mod test;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Playground {
    graph: Graph,
    focus_state: FocusState,
}

impl Playground {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_global_style.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let mut result = Self::default();
        let surface_id = result.graph.execute(AddSurface(AreaBounds::new(
            AxisBounds::new(50, 450),
            AxisBounds::new(50, 450),
        )));
        let surface = Query::new(&result.graph, surface_id);
        let surface_bounds = surface.bounds();
        let west_side = *surface.to_side(WEST);
        let east_side = *surface.to_side(EAST);
        let north_side = *surface.to_side(NORTH);
        result
            .graph
            .try_execute(AddTile {
                destination: west_side,
                intended_bounds: surface_bounds[Horizontal],
            })
            .unwrap();

        println!("{:#?}", result.graph);
        result
            .graph
            .try_execute(AddTile {
                destination: east_side,
                intended_bounds: AxisBounds::new(100, 450),
            })
            .unwrap();

        println!("{:#?}", result.graph);
        // result
        //     .graph
        //     .try_execute(AddTile {
        //         destination: east_side,
        //         intended_bounds: AxisBounds::new(300, 450),
        //     })
        //     .unwrap();
        // result
        // .graph
        // .try_execute(AddTile {
        //     destination: north_side,
        //     intended_bounds: AxisBounds::new(50, 150),
        // })
        // .unwrap();
        // println!("{}", serde_json::to_string_pretty(&result.graph).unwrap());
        result
    }
}

impl eframe::App for Playground {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                let (resp, painter) = ui.allocate_painter(
                    Vec2::new(ui.available_width(), ui.available_height()),
                    Sense::click_and_drag(),
                );
                let canvas = resp.rect;

                let pixels_per_point = ui.pixels_per_point();
                let pixel_size = 1.0 / pixels_per_point;

                for node in self
                    .graph
                    .iter()
                    .filter(|node| node.kind() == ElementKind::Surface)
                {
                    let bounds = node.bounds();
                    let shape = RectShape::new(
                        Rect::from_x_y_ranges(
                            Rangef::new(
                                bounds[Horizontal][Negative] as f32,
                                bounds[Horizontal][Positive] as f32,
                            ),
                            Rangef::new(
                                bounds[Vertical][Negative] as f32,
                                bounds[Vertical][Positive] as f32,
                            ),
                        ),
                        CornerRadius::ZERO,
                        Color32::BLACK,
                        Stroke::new(2f32, Color32::RED),
                        StrokeKind::Outside,
                    );
                    // println!("{:}: {:} -> {:}", *node, node.kind(), bounds);
                    //
                    // println!(
                    //     "{NORTH} {:}",
                    //     Query::<NeighbourTiles>::from(node.to_side(NORTH))
                    //         .into_iter()
                    //         .map(|node| *node)
                    //         .format(",")
                    // );
                    // println!(
                    //     "{SOUTH} {:}",
                    //     Query::<NeighbourTiles>::from(node.to_side(SOUTH))
                    //         .into_iter()
                    //         .map(|node| *node)
                    //         .format(",")
                    // );
                    // println!(
                    //     "{WEST} {:}",
                    //     Query::<NeighbourTiles>::from(node.to_side(EAST))
                    //         .into_iter()
                    //         .map(|node| *node)
                    //         .format(",")
                    // );
                    // println!(
                    //     "{EAST} {:}",
                    //     Query::<NeighbourTiles>::from(node.to_side(WEST))
                    //         .into_iter()
                    //         .map(|node| *node)
                    //         .format(",")
                    // );
                    painter.add(shape);
                }
                for node in self
                    .graph
                    .iter()
                    .filter(|node| node.kind() == ElementKind::Tile)
                {
                    let bounds = node.bounds();
                    let shape = RectShape::new(
                        Rect::from_x_y_ranges(
                            Rangef::new(
                                bounds[Horizontal][Negative] as f32,
                                bounds[Horizontal][Positive] as f32,
                            ),
                            Rangef::new(
                                bounds[Vertical][Negative] as f32,
                                bounds[Vertical][Positive] as f32,
                            ),
                        ),
                        CornerRadius::ZERO,
                        Color32::TRANSPARENT,
                        Stroke::new(1f32, Color32::WHITE),
                        StrokeKind::Inside,
                    );
                    // println!("{:}: {:} -> {:}", *node, node.kind(), bounds);
                    painter.add(shape);
                }
            });
        });
    }
}
