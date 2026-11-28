// use serde::{Deserialize, Serialize};
// use geotm_core::area::Area;
// use geotm_core::command::add_tile::AddTile;
// use geotm_core::command::transform::{TransformError, TransformInformation};
// use geotm_core::context::QueryContext;
// use geotm_core::error::KeyedError;
// use geotm_core::geometry::AxisBounds;
// use geotm_core::graph::{Graph, NodeId};
// use geotm_core::line::Line;
// use geotm_core::query::{ElementInformation, ElementInformation2D, ElementKind, GraphQuery, Query};
//
// /// todo: will probably move extras like this into a utils crate; It will be OK with the orphaning rules as long as Auto is owned in util.
// #[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
// struct Auto;
//
// type Test = Auto;
//
// impl QueryContext<AddTile<Line, Auto>> for Graph {
//     type OutKey = ();
//     type ErrorDetail = ();
//
//     fn resolve(&self, key: AddTile<Line, Auto>) -> Result<Query<Self::OutKey>, KeyedError<AddTile<Line, Auto>, Self::ErrorDetail>> {
//         todo!()
//     }
// }
//
// impl GraphQuery for Query<'_, Auto> {
//     type ValidationError = ();
// }
// impl TryFrom<Query<'static, Auto>> for Query<'static, AddTile<Line, Auto>> {
//     type Error = TransformError;
//
//     fn try_from(value: Query<'static, AddTile<Area, Auto>>) -> Result<Self, Self::Error> {
//         let destination = value.to(value.destination);
//         let bounds = if destination.kind() == ElementKind::Surface {
//             TryInto::<Query<NodeId>>::try_into(destination)
//                 .unwrap()
//                 .bounds()[destination.side.axis]
//         } else if let Some(details) = value.surroundings() {
//             let (space, segment_count): (i32, i32) =
//                 details.iter().fold((0, 0), |acc, segmented| {
//                     (
//                         acc.0
//                             + segmented
//                             .segments
//                             .iter()
//                             .map(|&area| value.to(area).bounds()[destination.side.axis].extent())
//                             .sum::<i32>(),
//                         acc.1 + (segmented.segments.len() as i32),
//                     )
//                 });
//
//             let half_new_size = (space / (segment_count + 1)) / 2;
//             let location = destination.bounds().side_location;
//             AxisBounds::new(location - half_new_size, location + half_new_size)
//         } else {
//             Err(TransformError::IncompatibleSurroundings)?
//         };
//         Ok(value.to(AddTile {
//             destination: value.destination,
//             intended_bounds: bounds,
//         }))
//     }
// }
