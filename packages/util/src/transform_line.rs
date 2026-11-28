// use geotm_core::command::Command;
// use geotm_core::key::QueryKey;
// use geotm_core::line::Line;
// use geotm_macros::{Command, QueryKey};
// use serde::de::DeserializeOwned;
// use serde::{Deserialize, Serialize};
// use std::fmt::Debug;
// use geotm_core::command::transform_area::TransformArea;
// use geotm_core::query::Query;
//
// #[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey, Command)]
// pub struct TransformLine<K = Line, L = i32>
// where
//     K: QueryKey,
//     L: 'static + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned,
// {
//     pub target: K,
//     pub intended_location: L,
// }
//
// impl <'a> From<Query<'a, TransformLine>> for TransformArea {
//     fn from(value: TransformLine) -> Self {
//         todo!()
//     }
// }
