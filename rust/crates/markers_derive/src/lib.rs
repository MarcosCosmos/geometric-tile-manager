use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Expr, parse_macro_input};

#[proc_macro_derive(QueryKey, attributes(node))]
pub fn query_key_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let attribute = input
        .attrs
        .iter()
        .filter(|a| a.path().segments.len() == 1 && a.path().segments[0].ident == "node")
        .nth(0);

    let node_key = if let Some(attr) = attribute {
        let result: Expr = attr
            .parse_args()
            .expect("Expected an expression identifying the node key type.");
        quote!(#result)
    } else {
        quote!(Node)
    };
    quote! {
        impl #impl_generics QueryKey for #name #ty_generics #where_clause {
            type NodeKey = #node_key;
        }
    }
    .into()
}

#[proc_macro_derive(SelectionCommand)]
pub fn selection_command_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics Command for #name #ty_generics #where_clause {
        }
        impl #impl_generics SelectionCommand for #name #ty_generics #where_clause {
            type Key = Key;
        }
    }
    .into()
}

#[proc_macro_derive(Command)]
pub fn command_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics Command for #name #ty_generics #where_clause {
        }
    }
        .into()
}

#[proc_macro_derive(NodeKey)]
pub fn node_key_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics NodeKey for #name #ty_generics #where_clause {
        }
        impl #impl_generics QueryKey for #name #ty_generics #where_clause {
            type NodeKey = Self;
        }
    }
    .into()
}
