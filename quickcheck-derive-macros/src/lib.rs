use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(QuickCheck)]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    let arbitray_impl = match data {
        syn::Data::Struct(data_struct) => match data_struct.fields {
            syn::Fields::Named(fields_named) => {
                let field_arbitrary_generators = fields_named
                    .named
                    .iter()
                    .map(|field| {
                        let identifier = &field.ident;
                        let ty = &field.ty;
                        quote! {
                            #identifier: <#ty as ::quickcheck::Arbitrary>::arbitrary(g)
                        }
                    })
                    .collect::<Vec<_>>();
                quote! {
                    Self {
                        #(#field_arbitrary_generators),*
                    }
                }
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                let field_arbitrary_generators = fields_unnamed
                    .unnamed
                    .iter()
                    .map(|field| {
                        let ty = &field.ty;
                        quote! {
                            <#ty as ::quickcheck::Arbitrary>::arbitrary(g)
                        }
                    })
                    .collect::<Vec<_>>();
                quote! {
                    Self(#(#field_arbitrary_generators),*)
                }
            }
            syn::Fields::Unit => quote! {Self},
        },
        syn::Data::Enum(data_enum) => {
            let num_variants = data_enum.variants.len();
            let initialisers = data_enum
                .variants
                .iter()
                .map(|variant| {
                    (
                        &variant.ident,
                        match variant.fields.len() {
                            0 => quote! {},
                            _ => {
                                let field_arbitrary_generators = variant
                                    .fields
                                    .iter()
                                    .map(|field| {
                                        let ty = &field.ty;
                                        quote! {<#ty as ::quickcheck::Arbitrary>::arbitrary(g)}
                                    })
                                    .collect::<Vec<_>>();
                                quote! {(#(#field_arbitrary_generators),*)}
                            }
                        },
                    )
                })
                .map(|initialiser| {
                    let ident = initialiser.0;
                    let initialiser_list = initialiser.1;
                    quote! {Self::#ident #initialiser_list}
                })
                .enumerate()
                .map(|(index, constructor)| {
                    quote! {#index => #constructor}
                })
                .collect::<Vec<_>>();
            quote! {
                match <::core::primitive::usize as ::quickcheck::Arbitrary>::arbitrary(g) % #num_variants {
                    #(#initialisers),*,
                    _ => ::core::unreachable!()
                }
            }
        }
        syn::Data::Union(_) => {
            syn::Error::new_spanned(&ident, "Cannot derive QuickCheck for a union yet")
                .to_compile_error()
        }
    };
    let output = quote! {
        impl ::quickcheck::Arbitrary for #ident
        where
            #ident: ::core::clone::Clone {
            fn arbitrary(g: &mut ::quickcheck::Gen) -> Self {
                #arbitray_impl
            }
        }
    };
    output.into()
}
