use syn::Type;

use crate::utils;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArgType {
    Option,
    Normal,
}

pub struct TypedArg<'a> {
    ty: ArgType,
    inner: &'a Type,
}

impl<'a> TypedArg<'a> {
    pub fn new(ty: &'a Type) -> Self {
        if let Some(ty) =
            utils::extract_generic_type(ty, &["Option", "std:option:Option", "core:option:Option"])
        {
            TypedArg {
                ty: ArgType::Option,
                inner: ty,
            }
        } else {
            TypedArg {
                ty: ArgType::Normal,
                inner: ty,
            }
        }
    }

    pub fn inner(&self) -> &'_ Type {
        self.inner
    }

    pub fn ty(&self) -> ArgType {
        self.ty
    }
}
