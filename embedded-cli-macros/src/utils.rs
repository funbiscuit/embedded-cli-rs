use syn::{Generics, PathArguments, Type, TypePath};

use darling::{usage::GenericsExt, Error, Result};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

pub struct TargetType {
    has_lifetime: bool,
    ident: Ident,
}

impl TargetType {
    pub fn parse(ident: Ident, generics: Generics) -> Result<Self> {
        if generics.declared_lifetimes().len() > 1 {
            let mut accum = Error::accumulator();
            accum.extend(generics.lifetimes().skip(1).map(|param| {
                Error::custom(
                    "More than one lifetime parameter specified. Try removing this lifetime param.",
                )
                .with_span(param)
            }));
            accum.finish()?;
        }

        if !generics.declared_type_params().is_empty() {
            let mut accum = Error::accumulator();
            accum.extend(generics.type_params().map(|param| {
                Error::custom(
                    "Target type must not be generic over any type. Try removing thus type param.",
                )
                .with_span(param)
            }));
            accum.finish()?;
        }

        let has_lifetime = !generics.declared_lifetimes().is_empty();

        Ok(Self {
            has_lifetime,
            ident,
        })
    }

    pub fn ident(&self) -> &Ident {
        &self.ident
    }

    pub fn named_lifetime(&self) -> TokenStream {
        if self.has_lifetime {
            quote! {
                <'a>
            }
        } else {
            quote! {}
        }
    }

    pub fn unnamed_lifetime(&self) -> TokenStream {
        if self.has_lifetime {
            quote! {
                <'_>
            }
        } else {
            quote! {}
        }
    }
}

pub fn extract_generic_type<'a>(ty: &'a Type, expected_container: &[&str]) -> Option<&'a Type> {
    //TODO: rewrite
    // If it is not `TypePath`, it is not possible to be `Option<T>`, return `None`
    if let Type::Path(TypePath { qself: None, path }) = ty {
        // We have limited the 5 ways to write `Option`, and we can see that after `Option`,
        // there will be no `PathSegment` of the same level
        // Therefore, we only need to take out the highest level `PathSegment` and splice it into a string
        // for comparison with the analysis result
        let segments_str = &path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join(":");
        // Concatenate `PathSegment` into a string, compare and take out the `PathSegment` where `Option` is located
        let option_segment = expected_container
            .iter()
            .find(|s| segments_str == *s)
            .and_then(|_| path.segments.last());
        let inner_type = option_segment
            // Take out the generic parameters of the `PathSegment` where `Option` is located
            // If it is not generic, it is not possible to be `Option<T>`, return `None`
            // But this situation may not occur
            .and_then(|path_seg| match &path_seg.arguments {
                PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    args, ..
                }) => args.first(),
                _ => None,
            })
            // Take out the type information in the generic parameter
            // If it is not a type, it is not possible to be `Option<T>`, return `None`
            // But this situation may not occur
            .and_then(|generic_arg| match generic_arg {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => None,
            });
        // Return `T` in `Option<T>`
        return inner_type;
    }
    None
}
