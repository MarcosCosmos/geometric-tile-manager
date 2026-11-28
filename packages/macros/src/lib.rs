use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    Data, DataStruct, DeriveInput, Expr, Fields, FieldsNamed, FieldsUnnamed, LitInt, LitStr,
    parse_macro_input,
};

#[proc_macro_derive(QueryKey)]
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

#[proc_macro_derive(Encode, attributes(encode))]
pub fn encode_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let fmt = input
        .attrs
        .iter()
        .find(|a| a.path().segments.len() == 1 && a.path().segments[0].ident == "encode")
        .map(|attr| {
            attr.parse_args::<LitStr>()
                .expect("Bad encode(\"...\") attribute: should be a string literal")
        })
        .expect("Missing required attribute encode(\"...\") for Display/Debug impl");

    let fields = match input.data {
        Data::Struct(DataStruct { fields, .. }) => match fields {
            Fields::Named(FieldsNamed { named, .. }) => named
                .iter()
                .map(|field| {
                    let name = field.ident.as_ref().unwrap();
                    quote! { #name=self.#name, }
                })
                .collect::<Vec<_>>(),
            Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => unnamed
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let i = LitInt::new(&i.to_string(), Span::call_site());
                    quote! { self.#i, }
                })
                .collect::<Vec<_>>(),
            Fields::Unit => Vec::new(),
        },
        _ => {
            panic!("This can only be derived on structs")
        }
    };
    quote! {
        #[automatically_derived]
        impl #impl_generics Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, #fmt, #(#fields)*)
            }
        }
        #[automatically_derived]
        impl #impl_generics Debug for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                Display::fmt(self, f)
            }
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
        #[automatically_derived]
        impl #impl_generics NodeKey for #name #ty_generics #where_clause {
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
