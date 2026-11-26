use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use uuid::Uuid;

pub trait QueryKey:
    Sized + Copy + Clone + Debug + Eq + PartialEq + Serialize + DeserializeOwned + 'static
{
    // type NodeKey: NodeKey;
}

// impl NodeKey for Uuid {}

impl QueryKey for Uuid {
    // type NodeKey = Self;
}
//
// pub trait NodeKey: QueryKey {}

#[macro_export]
macro_rules! query_impl {
    ($(<$($generics:ident),*>)? $trait_type:ident<$($source:ty)|+> | $($extra_trait:ident<$($extra_source:ty)|+>)|+ for $dest:ident$(<$($dest_generics:ident),+>)? $(where $($clause:tt)+)?) => {
        query_impl!{$(<$($generics),*>)? $trait_type<$($source)|+> for $dest$(<$($dest_generics:ident),+>)? $(where $($clause)+)?}
        query_impl!{$(<$($generics),*>)? $($extra_trait<$($extra_source)|+>)|+ for $dest$(<$($dest_generics:ident),+>)? $(where $($clause)+)?}
    };
    ($(<$($generics:ident),*>)? $trait_type:ident<$source:ty | $($extra_source:ty)|+> for $dest:ident$(<$($dest_generics:ident),+>)? $(where $($clause:tt)+)?) => {
        query_impl!{$(<$($generics),*>)? $trait_type<$source> for $dest $(where $($clause),+)?}
        query_impl!{$(<$($generics),*>)? $trait_type<$($extra_source)|+> for $dest $(where $($clause)+)?}
    };
    ($(<$($generics:ident),*>)? From<$source:ty> for $dest:ident$(<$($dest_generics:ident),+>)? $(where $($clause:tt)+)?) => {
        impl<'a$($(,$generics)*)?> From<Query<'a, $source>> for Query<'a, $dest> $(where $($clause)+)?
        {
            fn from(value: Query<'a, $source>) -> Self {
                use std::ops::Deref;
                Query::new(value.context, $dest$(::<$($dest_generics),+>)?::from(*(value.deref())))
            }
        }
    };
    ($(<$($generics:ident),*>)? TryFrom<$source:ty> for $dest:ident$(<$($dest_generics:ident),+>)? $(where $($clause:tt)+)?) => {
        impl<'a$($(,$generics)*)?> TryFrom<Query<'a, $source>> for Query<'a, $dest> $(where $($clause)+)?
        {
            type Error = <$dest as TryFrom<$source>>::Error;
            fn try_from(value: Query<'a, $source>) -> Result<Self, Self::Error> {
                $dest$(::<$($dest_generics),+>)?::try_from(*value).map(|dest| Query::new(value.context, dest))
            }
        }
    };
}
pub(crate) use query_impl;
