use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::{DeriveInput, Expr, Ident, Result, Token, parse_macro_input};

#[proc_macro_derive(QueryKey, attributes(query))]
pub fn query_key_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let attribute = input
        .attrs
        .iter()
        .filter(|a| a.path().segments.len() == 1 && a.path().segments[0].ident == "query")
        .nth(0)
        .expect("Can't derive QueryKey: missing required query_key(NodeKey) attribute.");

    let node_key: Expr = attribute
        .parse_args()
        .expect("Expected an expression identifying the node key type.");
    quote! {
        impl #impl_generics QueryKey for #name #ty_generics #where_clause {
            type NodeKey = #node_key;
        }
    }
    .into()
}
