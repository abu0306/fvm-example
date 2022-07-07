use crate::ir::{
    attrs,
    attrs::Attrs as _,
};

use crate::error::ExtError as _;
use super::{
    constructor,
    message,
};

use crate::{
    format_err_spanned,
    format_err,
};

use syn::spanned::Spanned as _;

#[derive(Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum ImplItem {
    Constructor(constructor::Constructor),
    Message(message::Message),
    Other(syn::ImplItem),
}

impl quote::ToTokens for ImplItem {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Constructor(constructor) => constructor.to_tokens(tokens),
            Self::Message(message) => message.to_tokens(tokens),
            Self::Other(other) => other.to_tokens(tokens),
        }
    }
}


impl TryFrom<syn::ImplItem> for ImplItem {
    type Error = syn::Error;

    fn try_from(impl_item: syn::ImplItem) -> Result<Self, Self::Error> {
        match impl_item {
            syn::ImplItem::Method(method_item) => {
                if !attrs::contains_fvm_attributes(&method_item.attrs) {
                    return Ok(Self::Other(method_item.into()));
                }
                let attr = attrs::first_fvm_attribute(&method_item.attrs)?
                    .expect("missing expected fvm attribute for struct");
                match attr.first().kind() {
                    attrs::AttributeArg::Message => {
                        <message::Message as TryFrom<_>>::try_from(method_item)
                            .map(Into::into)
                            .map(Self::Message)
                    }
                    attrs::AttributeArg::Constructor => {
                        <constructor::Constructor as TryFrom<_>>::try_from(method_item)
                            .map(Into::into)
                            .map(Self::Constructor)
                    }
                    _ => Err(format_err_spanned!(
                        method_item,
                        "encountered invalid fvm attribute at this point, expected either \
                        #[fvm(message)] or #[fvm(constructor) attributes"
                    )),
                }
            }
            other_item => {
                if attrs::contains_fvm_attributes(other_item.attrs()) {
                    let (fvm_attrs, _) =
                        attrs::partition_attributes(other_item.attrs().iter().cloned())?;
                    assert!(!fvm_attrs.is_empty());
                    fn into_err(attr: &attrs::FvmAttribute) -> syn::Error {
                        format_err!(attr.span(), "encountered unexpected fvm attribute",)
                    }
                    return Err(fvm_attrs[1..]
                        .iter()
                        .map(into_err)
                        .fold(into_err(&fvm_attrs[0]), |fst, snd| fst.into_combine(snd)));
                }
                Ok(Self::Other(other_item))
            }
        }
    }
}


impl ImplItem {
    pub fn is_message(&self) -> bool {
        self.filter_map_message().is_some()
    }


    pub fn filter_map_message(&self) -> Option<&message::Message> {
        match self {
            ImplItem::Message(message) => Some(message),
            _ => None,
        }
    }

    pub fn is_constructor(&self) -> bool {
        self.filter_map_constructor().is_some()
    }


    pub fn filter_map_constructor(&self) -> Option<&constructor::Constructor> {
        match self {
            ImplItem::Constructor(constructor) => Some(constructor),
            _ => None,
        }
    }

    pub fn is_other_item(&self) -> bool {
        self.filter_map_other_item().is_some()
    }

    pub fn filter_map_other_item(&self) -> Option<&syn::ImplItem> {
        match self {
            ImplItem::Other(rust_item) => Some(rust_item),
            _ => None,
        }
    }
}