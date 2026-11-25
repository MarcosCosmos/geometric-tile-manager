use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(QueryKey, attributes(node))]
pub fn query_key_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        #[automatically_derived]
        impl #impl_generics QueryKey for #name #ty_generics #where_clause {
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
        #[automatically_derived]
        impl #impl_generics Command for #name #ty_generics #where_clause {
        }
    }
    .into()
}

#[proc_macro_derive(ValidationError)]
pub fn validation_error_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        #[automatically_derived]
        impl #impl_generics ValidationError for #name #ty_generics #where_clause {
        }
    }
    .into()
}
