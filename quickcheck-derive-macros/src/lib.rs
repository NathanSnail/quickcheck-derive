use proc_macro::{self, TokenStream};
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{DeriveInput, Field, Ident, Type, parse_macro_input, punctuated::Punctuated};

fn generate_product_shrink<T>(
    fields: &Punctuated<Field, T>,
    constructor: impl Fn(
        &Type,
        &Ident,
        &Vec<(Ident, proc_macro2::TokenStream)>,
    ) -> proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let self_copies = fields
        .iter()
        .map(|field| {
            let ident = field
                .ident
                .clone()
                .expect("Named identifier must have an identifier");
            let unique_self = format_ident!("self_{}", ident);
            quote! {
                let #unique_self = <Self as ::std::clone::Clone>::clone(&self);
            }
        })
        .collect::<Vec<_>>();

    let cloning_iterator_madness = fields
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            let ident = Ident::new(
                &field
                    .ident
                    .clone()
                    .map(|ident| ident.to_string())
                    .unwrap_or(idx.to_string()),
                Span::call_site(),
            );
            let other_idents = fields
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    Ident::new(
                        &field
                            .ident
                            .clone()
                            .map(|ident| ident.to_string())
                            .unwrap_or(idx.to_string()),
                        Span::call_site(),
                    )
                })
                .filter(|e| e != &ident)
                .map(|field_ident| {
                    let unique_self = format_ident!("self_{}", ident);
                    (
                        field_ident.clone(),
                        quote! {::core::clone::Clone::clone(&#unique_self.#field_ident)},
                    )
                })
                .collect::<Vec<_>>();
            constructor(&field.ty, &ident, &other_idents)
        })
        .rev()
        .reduce(|a, b| quote! {::std::iter::Iterator::chain(#a, #b)})
        .unwrap_or(quote! {});

    quote! {
        #(#self_copies)*
        ::std::boxed::Box::new(#cloning_iterator_madness)
    }
}

#[proc_macro_derive(QuickCheck)]
pub fn derive(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);
    let (shrink_impl, arbitray_impl) = match data {
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
                (
                    generate_product_shrink(&fields_named.named, |ty, ident, other_idents| {
                        let other_idents_initialisers = other_idents
                            .iter()
                            .map(|(ident, toks)| {
                                quote! {#ident: #toks}
                            })
                            .collect::<Vec<_>>();
                        quote! {
                            ::std::iter::Iterator::map(<#ty as ::quickcheck::Arbitrary>::shrink(&self.#ident),
                                move |e| Self {#ident: e, #(#other_idents_initialisers),*})
                        }
                    }),
                    quote! {
                        Self {
                            #(#field_arbitrary_generators),*
                        }
                    },
                )
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
                (
                    quote! {
                        ::quickcheck::empty_shrinker()
                    },
                    quote! {
                        Self(#(#field_arbitrary_generators),*)
                    },
                )
            }
            syn::Fields::Unit => (quote! {::quickcheck::empty_shrinker()}, quote! {Self}),
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
            (
                quote! {
                    ::quickcheck::empty_shrinker()
                },
                quote! {
                    match <::core::primitive::usize as ::quickcheck::Arbitrary>::arbitrary(g) % #num_variants {
                        #(#initialisers),*,
                        _ => ::core::unreachable!()
                    }
                },
            )
        }
        syn::Data::Union(_) => (quote! {::quickcheck::empty_shrinker()}, {
            syn::Error::new_spanned(&ident, "Cannot derive QuickCheck for a union yet")
                .to_compile_error()
        }),
    };
    let output = quote! {
        impl ::quickcheck::Arbitrary for #ident
        where
            #ident: ::core::clone::Clone {
            fn arbitrary(g: &mut ::quickcheck::Gen) -> Self {
                #arbitray_impl
            }

            fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
                #shrink_impl
            }
        }
    };
    output.into()
}
