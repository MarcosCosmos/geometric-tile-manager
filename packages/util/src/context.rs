// use std::ops::{Deref, DerefMut};
// use serde::de::Error;
// use serde::{Deserialize, Serialize};
// use snafu::ResultExt;
// use geotm_core::command::{Command, CommandExecutor};
// use geotm_core::context::QueryContext;
// use geotm_core::error::{KeyedError, KeyedSnafu};
// use geotm_core::graph::Graph;
// use geotm_core::key::QueryKey;
// use geotm_core::query::Query;
//
// pub struct ExtendedCommandResolver<'a>(pub &'a mut Graph);
//
// impl Deref for ExtendedCommandResolver<'_> {
//     type Target = Graph;
//
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }
//
// impl DerefMut for ExtendedCommandResolver<'_> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }
//
// /// A helper macro that creates a local Sugar<T> wrapper in this module for working around rust's orphan rules.
// /// Partly, the idea is that (de)serialisation can stay literal for various command types/args.
// /// More of a proof of concept that the geotm-core lib can be specialised across crates in this way than anything else.
// #[macro_export]
// macro_rules! marker {
//     ($name:ident) => {
//         use geotm_macros::{QueryKey, Command};
//         #[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, QueryKey, Command)]
//         pub struct $name<T>(pub T) where T: QueryKey + Command;
//     };
// }
//
// marker!(Sugar );
//
//
// /// note: unlike TryDesugar, this method assumes the input command is already valid. Mainly intended as a utility for coalescing multiple TryDesugars
// pub trait Desugar<C>
//     where C: QueryKey + Command
// {
//     type OutKey: QueryKey + Command;
//     fn desugar(&self, command: C) -> Self::OutKey;
// }
//
// pub trait TryDesugar<C>
// where C: QueryKey + Command
// {
//     type OutKey: QueryKey + Command;
//     type Error: Error + 'static;
//     fn try_desugar(&self, command: C) -> Result<Self::OutKey, Self::Error>;
// }
//
// impl <K> QueryContext<K> for ExtendedCommandResolver<'_> where K: QueryKey + Command, Self : Desugar<Sugar<K>> {
//     type OutKey = <Self as Desugar<K>>::OutKey;
//     type ErrorDetail = <Graph as QueryContext<Sugar<K>>>::ErrorDetail;
//
//     fn resolve(&self, key: K) -> Result<Self::OutKey, KeyedError<K, Self::ErrorDetail>> {
//         Ok(self.desugar(Sugar(key)).with_context(|_| KeyedSnafu { key })?)
//     }
// }
//
// impl <C> CommandExecutor<C> for ExtendedCommandResolver<'_> where C: QueryKey + Command, Graph : CommandExecutor<C> {
//     type Response = <Graph as CommandExecutor<C>>::Response;
//     type ErrorDetail = <Graph as CommandExecutor<C>>::ErrorDetail;
//
//     fn try_execute(&mut self, command: C) -> Result<Self::Response, KeyedError<C, Self::ErrorDetail>> {
//         self.0.try_execute(command)
//     }
// }
