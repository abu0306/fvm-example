use crate::{
    ir::attrs,
};
use proc_macro2::{
    Ident,
    Span,
};
use syn::spanned::Spanned as _;
use crate::{
    format_err_spanned,
};

#[derive(Debug, PartialEq, Eq)]
pub struct Event {
    item: syn::ItemStruct,
    pub anonymous: bool,
}


impl quote::ToTokens for Event {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.item.to_tokens(tokens)
    }
}


impl TryFrom<syn::ItemStruct> for Event {
    type Error = syn::Error;

    fn try_from(item_struct: syn::ItemStruct) -> Result<Self, Self::Error> {
        let struct_span = item_struct.span();
        let (fvm_attrs, other_attrs) = attrs::sanitize_attributes(
            struct_span,
            item_struct.attrs,
            &attrs::AttributeArgKind::Event,
            |arg| {
                match arg.kind() {
                    attrs::AttributeArg::Event | attrs::AttributeArg::Anonymous => Ok(()),
                    _ => Err(None),
                }
            },
        )?;
        if !item_struct.generics.params.is_empty() {
            return Err(format_err_spanned!(
                item_struct.generics.params,
                "generic fvm! event structs are not supported",
            ));
        }
        Ok(Self {
            item: syn::ItemStruct {
                attrs: other_attrs,
                ..item_struct
            },
            anonymous: fvm_attrs.is_anonymous(),
        })
    }
}

impl Event {
    pub fn ident(&self) -> &Ident {
        &self.item.ident
    }

    pub fn fields(&self) -> EventFieldsIter {
        EventFieldsIter::new(self)
    }

    pub fn attrs(&self) -> &[syn::Attribute] {
        &self.item.attrs
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EventField<'a> {
    pub is_topic: bool,
    field: &'a syn::Field,
}

impl<'a> EventField<'a> {
    pub fn span(self) -> Span {
        self.field.span()
    }

    pub fn attrs(self) -> Vec<syn::Attribute> {
        let (_, non_fvm_attrs) = attrs::partition_attributes(self.field.attrs.clone())
            .expect("encountered invalid event field attributes");
        non_fvm_attrs
    }

    pub fn vis(self) -> &'a syn::Visibility {
        &self.field.vis
    }

    pub fn ident(self) -> Option<&'a Ident> {
        self.field.ident.as_ref()
    }

    pub fn ty(self) -> &'a syn::Type {
        &self.field.ty
    }
}

pub struct EventFieldsIter<'a> {
    iter: syn::punctuated::Iter<'a, syn::Field>,
}

impl<'a> EventFieldsIter<'a> {
    fn new(event: &'a Event) -> Self {
        Self {
            iter: event.item.fields.iter(),
        }
    }
}

impl<'a> Iterator for EventFieldsIter<'a> {
    type Item = EventField<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            None => None,
            Some(field) => {
                let is_topic = attrs::first_fvm_attribute(&field.attrs)
                    .unwrap_or_default()
                    .map(|attr| matches!(attr.first().kind(), attrs::AttributeArg::Topic))
                    .unwrap_or_default();
                Some(EventField { is_topic, field })
            }
        }
    }
}
