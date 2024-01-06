use darling::{Error, Result};
use proc_macro2::Ident;
use syn::{Fields, Type, Variant};

pub struct CommandGroup {
    ident: Ident,
    field_type: Type,
}

impl CommandGroup {
    pub fn parse(variant: &Variant) -> Result<Self> {
        let variant_ident = &variant.ident;

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
        })
    }

    pub fn ident(&self) -> &Ident {
        &self.ident
    }

    pub fn field_type(&self) -> &Type {
        &self.field_type
    }
}
