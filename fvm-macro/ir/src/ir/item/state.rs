use crate::format_err_spanned;
use crate::ir::{
    attrs,
};
use proc_macro2::Ident;
use syn::spanned::Spanned as _;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct State {
    ast: syn::ItemStruct,
}

impl quote::ToTokens for State {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.ast.to_tokens(tokens)
    }
}

impl TryFrom<syn::ItemStruct> for State {
    type Error = syn::Error;

    fn try_from(item_struct: syn::ItemStruct) -> Result<Self, Self::Error> {
        let struct_span = item_struct.span();

        let (_, other_attrs) = attrs::sanitize_attributes(
            struct_span,
            item_struct.attrs,
            &attrs::AttributeArgKind::State,
            |arg| {
                match arg.kind() {
                    attrs::AttributeArg::State => Ok(()),
                    _ => Err(None),
                }
            },
        )?;
        if !item_struct.generics.params.is_empty() {
            return Err(format_err_spanned!(
                item_struct.generics.params,
                "generic fvm state structs are not supported",
            ));
        }

        Ok(Self {
            ast: syn::ItemStruct {
                attrs: other_attrs,
                ..item_struct
            },
        })
    }
}

impl State {
    pub fn attrs(&self) -> &[syn::Attribute] {
        &self.ast.attrs
    }

    pub fn ident(&self) -> &Ident {
        &self.ast.ident
    }

    pub fn fields(&self) -> syn::punctuated::Iter<syn::Field> {
        self.ast.fields.iter()
    }
}
