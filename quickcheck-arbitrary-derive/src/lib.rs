use core::panic;
use std::collections::HashMap;

use proc_macro::{self};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, DataEnum, DeriveInput, Field, FieldsNamed, FieldsUnnamed, Ident, LitInt,
    MetaNameValue, Type, parse_macro_input, punctuated::Punctuated, token::Comma,
};

fn generate_product_shrink<
    Iter: IntoIterator<Item = Field> + Clone,
    IdentKind: Clone + ToTokens + ToString,
>(
    fields: &Iter,
    constructor: impl Fn(&Type, &IdentKind, &Vec<(IdentKind, TokenStream)>) -> TokenStream,
    make_ident: impl Fn(&str) -> IdentKind,
    self_helper: impl Fn(Ident, &IdentKind, usize) -> TokenStream,
) -> TokenStream {
    let self_copies = fields
        .clone()
        .into_iter()
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

    let cloning_iterator_madness: TokenStream = fields
        .clone()
        .into_iter()
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
                .clone()
                .into_iter()
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
                    let unique_self_toks = self_helper(
                        format_ident!("self_{}", ident.to_string()),
                        &field_ident,
                        fields.clone().into_iter().collect::<Vec<_>>().len(),
                    );
                    (
                        field_ident.clone(),
                        quote! {::core::clone::Clone::clone(#unique_self_toks)},
                    )
                })
                .collect::<Vec<_>>();
            constructor(&field.ty, &ident, &other_idents)
        })
        .collect::<Vec<_>>()
        .iter()
        .rev()
        .cloned()
        .reduce(|a, b| quote! {::std::iter::Iterator::chain(#a, #b)})
        .unwrap_or(quote! {});

    quote! {
        #(#self_copies)*
        ::std::boxed::Box::new(#cloning_iterator_madness)
    }
}
fn generate_product_shrink_simple<
    Iter: IntoIterator<Item = Field> + Clone,
    IdentKind: Clone + ToTokens + ToString,
>(
    fields: &Iter,
    constructor: impl Fn(&Type, &IdentKind, &Vec<(IdentKind, TokenStream)>) -> TokenStream,
    make_ident: impl Fn(&str) -> IdentKind,
) -> TokenStream {
    generate_product_shrink(
        fields,
        constructor,
        make_ident,
        |unique_self, field_ident, _| quote! {&#unique_self.#field_ident},
    )
}

fn make_enum_puller(pull: usize, others: usize, variant: &Ident, source: &Ident) -> TokenStream {
    let v_puller = [quote! {__quickcheck_derive_match_puller}];
    let pull_pattern = (0..(pull))
        .map(|_| quote! {_})
        .chain(v_puller.iter().cloned())
        .chain((pull..others).map(|_| quote! {_}));

    quote! {if let Self::#variant(#(#pull_pattern),*) = &#source {
        __quickcheck_derive_match_puller
    } else {
        ::core::unreachable!()
    }}
}

struct ArbitraryImpl {
    arbitrary: TokenStream,
    shrink: TokenStream,
}

fn make_named_struct_arbitrary(fields_named: &FieldsNamed) -> ArbitraryImpl {
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
    ArbitraryImpl {
        shrink: generate_product_shrink_simple(
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
                        move |__quickcheck_derive_moving| Self {#ident: __quickcheck_derive_moving, #(#other_idents_initialisers),*})
                }
            },
            |ident_str| Ident::new(ident_str, Span::call_site()),
        ),
        arbitrary: quote! {
            Self {
                #(#field_arbitrary_generators),*
            }
        },
    }
}

fn make_unnamed_struct_arbitrary(fields_unnamed: &FieldsUnnamed) -> ArbitraryImpl {
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
    ArbitraryImpl {
        arbitrary: quote! {
            Self(#(#field_arbitrary_generators),*)
        },
        shrink: generate_product_shrink_simple::<_, LitInt>(
            &fields_unnamed.unnamed,
            |ty, ident, other_idents| {
                let mut idents_all = other_idents.clone();
                idents_all.push((ident.clone(), quote! {__quickcheck_derive_moving}));
                idents_all.sort_by(|(a, _), (b, _)| {
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
                        move |__quickcheck_derive_moving| Self(#(#initialiser_list),*))
                }
            },
            |ident_str| LitInt::new(ident_str, Span::call_site()),
        ),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum RecursiveKind {
    None = 0,
    Linear = 1,
    Exponential = 2,
}

#[derive(Clone, Copy, Debug)]
struct EnumAtrributes {
    recursive: RecursiveKind,
}

fn get_enum_attrs(attrs: &Vec<Attribute>) -> EnumAtrributes {
    const RECURSION_INVALID_KIND: &str =
        "quickcheck recursive strategies must be one of None, Linear, Exponential";

    let all_attrs = attrs
        .iter()
        .filter(|attr| attr.meta.path().is_ident("quickcheck"))
        .map(|attr| {
            attr.parse_args_with(Punctuated::<MetaNameValue, Comma>::parse_terminated)
                .expect("quickcheck attribute must have comma seperated arguments")
                .iter()
                .map(|arg| {
                    (
                        arg.path
                            .get_ident()
                            .expect("quickcheck arguments must be of the form `ident = value`")
                            .to_string(),
                        match &arg.value {
                            syn::Expr::Path(v) => v.path.require_ident().expect("quickcheck recursive strategies must be one of None, Linear, Exponential").to_string(),
                            _ => panic!("quickcheck values must be literals"),
                        },
                    )
                })
                .collect::<HashMap<_, _>>()
        })
        .map(|key_values| EnumAtrributes {
            recursive: match key_values
                .get("recursive")
                .cloned()
            {
                Some(v) => match v.as_str() {
                    "None" => RecursiveKind::None,
                    "Linear" => RecursiveKind::Linear,
                    "Exponential" => RecursiveKind::Exponential,
                    _ => panic!("{}", RECURSION_INVALID_KIND)
                },
                None => RecursiveKind::None,
            },
        })
        .collect::<Vec<_>>();

    match all_attrs.len() {
        0 => EnumAtrributes {
            recursive: RecursiveKind::None,
        },
        1 => all_attrs[0],
        _ => panic!("quickcheck attribute may only be applied once to each field"),
    }
}

fn make_enum_arbitrary(ident: &Ident, data_enum: &DataEnum) -> ArbitraryImpl {
    let num_variants = data_enum.variants.len();

    let mut initialisers = data_enum
        .variants
        .iter()
        .map(|variant| {
            (
                &variant.ident,
                match variant.fields.len() {
                    0 => (quote! {}, RecursiveKind::None),
                    _ => {
                        let attrs = get_enum_attrs(&variant.attrs);
                        let new_g = match attrs.recursive {
                            RecursiveKind::Exponential => quote! {&mut ::quickcheck::Gen::new(::std::cmp::max(::quickcheck::Gen::size(g) / 2, 1))},
                            RecursiveKind::Linear => quote! {&mut ::quickcheck::Gen::new(::std::cmp::max(::quickcheck::Gen::size(g) - 1, 1))},
                            RecursiveKind::None => quote! {g}
                        };
                        let field_arbitrary_generators = variant
                            .fields
                            .iter()
                            .map(|field| {
                                let ty = &field.ty;
                                quote! {<#ty as ::quickcheck::Arbitrary>::arbitrary(#new_g)}
                            })
                            .collect::<Vec<_>>();
                        (quote! {(#(#field_arbitrary_generators),*)}, attrs.recursive)
                    }
                },
            )
        })
        .map(|(ident, (initialiser_list, recursive))| {
            (quote! {Self::#ident #initialiser_list}, recursive)
        })
        .enumerate()
        .map(|(index, (constructor, recursive))| {
            (quote! {#index => #constructor}, recursive)
        })
        .collect::<Vec<_>>();

    initialisers.sort_by_key(|(_, recursive)| *recursive);
    let num_recursive = initialisers
        .iter()
        .filter(|(_, recursive)| !matches!(recursive, RecursiveKind::None))
        .count();
    let initialisers = initialisers
        .into_iter()
        .map(|(toks, _)| toks)
        .collect::<Vec<_>>();

    let enum_name = &ident;
    let arm_matchers = data_enum
        .variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            let shrinker = generate_product_shrink::<_, LitInt>(
                &variant.fields,
                |ty, ident, other_idents| {
                    let mut idents_all = other_idents.clone();
                    idents_all.push((ident.clone(), quote! {__quickcheck_derive_moving}));
                    idents_all.sort_by(|(a, _), (b, _)| {
                        a.base10_parse::<u64>()
                            .unwrap()
                            .cmp(&b.base10_parse().unwrap())
                    });
                    let initialiser_list = idents_all
                        .iter()
                        .map(|(_, stream)| stream)
                        .collect::<Vec<_>>();

                    let puller = make_enum_puller(
                        ident.base10_parse().unwrap(),
                        other_idents.len(),
                        &variant.ident,
                        &Ident::new("self", Span::call_site()),
                    );

                    quote! {
                        ::std::iter::Iterator::map(<#ty as ::quickcheck::Arbitrary>::shrink(
                            #puller
                        ),
                        move |__quickcheck_derive_moving| Self::#variant_ident(#(#initialiser_list),*))
                    }
                },
                |ident_str| LitInt::new(ident_str, Span::call_site()),
                |ident, field, num_fields| {
                   make_enum_puller(
                        field.base10_parse().unwrap(),
                        num_fields - 1,
                        variant_ident,
                        &ident,
                    )
                },
            );

            let underscores = (0..variant.fields.len())
                .map(|_| quote! {_})
                .collect::<Vec<_>>();

             match variant.fields.is_empty() {
                true => quote! {#enum_name::#variant_ident => ::std::boxed::Box::new(::quickcheck::empty_shrinker())},
                false => quote! {#enum_name::#variant_ident(#(#underscores),*) => {#shrinker}} ,
            }

        })
        .collect::<Vec<_>>();

    ArbitraryImpl {
        arbitrary: quote! {
            match <::core::primitive::usize as ::quickcheck::Arbitrary>::arbitrary(g) % (
            if ::quickcheck::Gen::size(g) > 1 {
                #num_variants
            } else {
                #num_variants - #num_recursive
            }) {
                #(#initialisers),*,
                _ => ::core::unreachable!()
            }
        },
        shrink: quote! {
            match &self {
                #(#arm_matchers),*
            }
        },
    }
}

/// Generates an implementation of `quickcheck::Arbitrary`.
///
/// You can annotate an enum variant with `#[quickcheck(recursive = None | Linear | Exponential)]` to allow for testing of potentially infinitely large types
///
/// ```rs
/// #[derive(Clone, QuickCheck, Debug)]
/// enum Tree<T> {
///     #[quickcheck(recursive = Exponential)]
///     Branch(Vec<Tree<T>>),
///     Leaf(T),
/// }
/// ```
///
/// Use exponential for types that exponentially grow with depth (like trees).
///
/// Use linear for types that linearly grow with depth (like linked lists).
#[proc_macro_derive(QuickCheck, attributes(quickcheck))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput {
        ident,
        data,
        generics,
        ..
    } = parse_macro_input!(input);
    let ArbitraryImpl { arbitrary, shrink } = match data {
        syn::Data::Struct(data_struct) => match data_struct.fields {
            syn::Fields::Named(fields_named) => make_named_struct_arbitrary(&fields_named),
            syn::Fields::Unnamed(fields_unnamed) => make_unnamed_struct_arbitrary(&fields_unnamed),
            syn::Fields::Unit => ArbitraryImpl {
                arbitrary: quote! {Self},
                shrink: quote! {::quickcheck::empty_shrinker()},
            },
        },
        syn::Data::Enum(data_enum) => make_enum_arbitrary(&ident, &data_enum),
        syn::Data::Union(_) => ArbitraryImpl {
            shrink: quote! {::quickcheck::empty_shrinker()},
            arbitrary: {
                syn::Error::new_spanned(&ident, "Cannot derive QuickCheck for a union yet")
                    .to_compile_error()
            },
        },
    };

    let generics_unconstrained = generics
        .lifetimes()
        .map(|lifetime| lifetime.lifetime.to_token_stream())
        .chain(
            generics
                .type_params()
                .map(|type_param| type_param.ident.to_token_stream()),
        )
        .collect::<Vec<_>>();

    let generics_arbitrary = generics
        .lifetimes()
        .map(|lifetime| lifetime.to_token_stream())
        .chain(generics.type_params().map(|type_param| {
            let colon = match type_param.bounds.len() {
                0 => quote! {:},
                _ => quote! {+},
            };
            quote! {#type_param #colon ::quickcheck::Arbitrary}
        }))
        .collect::<Vec<_>>();

    let generics_unconstrained_tokens = match generics_unconstrained.len() {
        0 => quote! {},
        _ => quote! {<#(#generics_unconstrained),*>},
    };
    let generics_arbitrary_tokens = match generics_arbitrary.len() {
        0 => quote! {},
        _ => quote! {<#(#generics_arbitrary),*>},
    };

    if !generics.lifetimes().collect::<Vec<_>>().is_empty() {
        return syn::Error::new_spanned(
            &ident,
            "Cannot derive QuickCheck for a type with lifetimes yet",
        )
        .to_compile_error()
        .into();
    }

    let output = quote! {
        impl #generics_arbitrary_tokens ::quickcheck::Arbitrary for #ident #generics_unconstrained_tokens
        where
            #ident #generics_unconstrained_tokens : ::core::clone::Clone {
            fn arbitrary(g: &mut ::quickcheck::Gen) -> Self {
                #arbitrary
            }

            fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
                #shrink
            }
        }
    };
    output.into()
}
