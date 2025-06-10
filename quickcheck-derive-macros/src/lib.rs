use std::u64;

use proc_macro::{self, TokenStream};
use proc_macro2::{Literal, Span};
use quote::{IdentFragment, ToTokens, format_ident, quote};
use syn::{DeriveInput, Field, Ident, LitInt, Type, parse_macro_input, punctuated::Punctuated};

fn generate_product_shrink<PunctKind, IdentKind: Clone + ToTokens + ToString>(
    fields: &Punctuated<Field, PunctKind>,
    constructor: impl Fn(
        &Type,
        &IdentKind,
        &Vec<(IdentKind, proc_macro2::TokenStream)>,
    ) -> proc_macro2::TokenStream,
    make_ident: impl Fn(&str) -> IdentKind,
) -> proc_macro2::TokenStream {
    let self_copies = fields
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            let ident = field
                .ident
                .clone()
                .map(|ident| ident.to_string())
                .unwrap_or(idx.to_string());
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
            let ident = make_ident(
                &field
                    .ident
                    .clone()
                    .map(|ident| ident.to_string())
                    .unwrap_or(idx.to_string()),
            );
            let other_idents = fields
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    make_ident(
                        &field
                            .ident
                            .clone()
                            .map(|ident| ident.to_string())
                            .unwrap_or(idx.to_string()),
                    )
                })
                .filter(|e| e.to_string() != ident.to_string())
                .map(|field_ident| {
                    let unique_self = format_ident!("self_{}", ident.to_string());
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
                    generate_product_shrink(
                        &fields_named.named,
                        |ty, ident, other_idents| {
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
                        },
                        |ident_str| Ident::new(ident_str, Span::call_site()),
                    ),
                    quote! {
                        Self {
                            #(#field_arbitrary_generators),*
                        }
                    },
                )
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                eprintln!("{:?}", fields_unnamed);
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
                    generate_product_shrink::<_, LitInt>(
                        &fields_unnamed.unnamed,
                        |ty, ident, other_idents| {
                            let mut idents_all = other_idents.clone();
                            idents_all.push((ident.clone(), quote! {e}));
                            idents_all.sort_by(|(a, _), (b, _)| {
                                eprintln!("{:?}", a.to_string());
                                a.base10_parse::<u64>()
                                    .unwrap()
                                    .cmp(&b.base10_parse().unwrap())
                            });
                            let initialiser_list = idents_all
                                .iter()
                                .map(|(_, stream)| stream)
                                .collect::<Vec<_>>();

                            quote! {
                                ::std::iter::Iterator::map(<#ty as ::quickcheck::Arbitrary>::shrink(&self.#ident),
                                    move |e| Self(#(#initialiser_list),*))
                            }
                        },
                        |ident_str| {
                            eprintln!(
                                "|{}, {}|",
                                &ident_str,
                                ident_str.parse::<u64>().unwrap_or(u64::MAX)
                            );
                            LitInt::new(ident_str, Span::call_site())
                        },
                    ),
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

#[proc_macro]
pub fn syntax_dump(input: TokenStream) -> TokenStream {
    eprintln!("{:#?}", &input);
    input
}
