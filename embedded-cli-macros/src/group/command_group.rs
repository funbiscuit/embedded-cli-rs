use darling::{Error, FromVariant, Result};
use proc_macro2::Ident;
use syn::{Fields, Type, Variant};

#[derive(Debug, FromVariant, Default)]
#[darling(default, attributes(group), forward_attrs(allow, doc, cfg))]
struct GroupAttrs {
    hidden: bool,
}

#[derive(Debug)]
pub struct CommandGroup {
    pub ident: Ident,
    pub field_type: Type,
    pub hidden: bool,
}

impl CommandGroup {
    pub fn parse(variant: &Variant) -> Result<Self> {
        let variant_ident = &variant.ident;
        let attrs = GroupAttrs::from_variant(variant)?;

        let field = match &variant.fields {
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    return Err(
                        Error::custom("Group variant must have a single tuple field")
                            .with_span(fields),
                    );
                }
                fields.unnamed.first().unwrap()
            }
            _ => {
                return Err(
                    Error::custom("Group variant must have a single tuple field")
                        .with_span(variant),
                )
            }
        };

        Ok(Self {
            ident: variant_ident.clone(),
            field_type: field.ty.clone(),
            hidden: attrs.hidden,
        })
    }
}
