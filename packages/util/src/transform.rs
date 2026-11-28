// use crate::context::{Desugar, ExtendedCommandResolver, Sugar, TryDesugar};
// use crate::relative::Relative;
// use geotm_core::command::transform_area::TransformArea;
// use geotm_core::context::QueryContext;
// use geotm_core::graph::Graph;
// use geotm_core::key::QueryKey;
// use geotm_core::line::{Line, LineQuery};
// use geotm_core::query::{ElementInformation2D, Query};
// use serde::de::DeserializeOwned;
// use serde::Serialize;
// use std::fmt::Debug;
//
// impl Desugar<Sugar<TransformArea<Line, i32>>> for ExtendedCommandResolver<'_>
// where
// {
//     type OutKey = TransformArea;
//
//     fn desugar(&self, sugar: Sugar<TransformArea<Line, i32>>) -> Self::OutKey {
//         let line = Query::new(self.0, sugar.0.target);
//         // here we assume that the line has already resolved, because the code for that is local to our sugar!
//         let area = line.to_area(|iter| iter.last()).unwrap();
//         let mut bounds = area.bounds()[line.side.axis];
//         bounds[line.side.sign] = sugar.0.intended_bounds;
//
//         TransformArea::new(*area, sugar.0.axis, bounds)
//     }
// }
//
// impl <K> TryDesugar<Sugar<TransformArea<K, Relative<i32>>>> for ExtendedCommandResolver<'_> where
//     K: QueryKey,
//     for <'a> Graph: QueryContext<K>,
//     for <'a> Query<'a, <Graph as QueryContext<K>>::OutKey>: Into<Query<'a, Line>>,
// {
//     type OutKey = TransformArea;
//
//     type Error = ();
//
//     fn try_desugar(&self, sugar: Sugar<TransformArea<K, Relative<i32>>>) -> Result<Self::OutKey, Self::Error> {
//         let line = self.0.resolve(sugar.0.target)?.into();
//
//
//         if line.side.axis !=
//
//         self.desugar(TransformArea::new())
//     }
// }
//
// impl <K, B> TryDesugar<Sugar<TransformArea<K, B>>> for ExtendedCommandResolver<'_> where
//     B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
// {
//     type OutKey = TransformArea;
//
//     type Error = ();
//
//     fn try_desugar(&self, sugar: Sugar<TransformArea<Line, B>>) -> Result<Self::OutKey, Self::Error> {
//         let line = self.0.resolve(sugar.0.target)?;
//         let area = line.to_area(|iter| iter.last())?;
//
//     }
// }
// //
// // impl <B> QueryContext<Transform<Line, B>> for ExtendedCommandResolver<'_> where
// //     B: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
// // {
// //     type OutKey = Transform;
// //     type ErrorDetail = !; // todo: update this error detail
// //
// //     fn resolve(&self, key: Transform<Line, B>) -> Result<Query<Self::OutKey>, KeyedError<Transform<Line, B>, Self::ErrorDetail>> {
// //         let line = self.0.resolve(key.target);
// //         self.resolve()
// //         Ok(Transform {
// //
// //         })
// //     }
// // }
