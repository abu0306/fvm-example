pub mod impl_item;
pub mod constructor;
pub mod callable;
pub mod message;
pub mod iter;

use super::{
    attrs,
    attrs::Attrs as _,
    item_impl::iter::{
        IterMessages,
        IterConstructors,
    },
};

use crate::{
    format_err_spanned,
    format_err,
};
use syn::spanned::Spanned as _;
use quote::TokenStreamExt as _;
use crate::error::ExtError as _;


#[derive(Debug, PartialEq, Eq)]
pub struct ItemImpl {
    attrs: Vec<syn::Attribute>,
    defaultness: Option<syn::token::Default>,
    unsafety: Option<syn::token::Unsafe>,
    impl_token: syn::token::Impl,
    generics: syn::Generics,
    trait_: Option<(Option<syn::token::Bang>, syn::Path, syn::token::For)>,
    self_ty: Box<syn::Type>,
    brace_token: syn::token::Brace,
    items: Vec<impl_item::ImplItem>,
    namespace: Option<attrs::Namespace>,
}

impl quote::ToTokens for ItemImpl {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append_all(
            self.attrs
                .iter()
                .filter(|attr| matches!(attr.style, syn::AttrStyle::Outer)),
        );
        self.defaultness.to_tokens(tokens);
        self.unsafety.to_tokens(tokens);
        self.impl_token.to_tokens(tokens);
        self.generics.to_tokens(tokens);
        if let Some((polarity, path, for_token)) = &self.trait_ {
            polarity.to_tokens(tokens);
            path.to_tokens(tokens);
            for_token.to_tokens(tokens);
        }
        self.self_ty.to_tokens(tokens);
        self.generics.where_clause.to_tokens(tokens);
        self.brace_token.surround(tokens, |tokens| {
            tokens.append_all(
                self.attrs
                    .iter()
                    .filter(|attr| matches!(attr.style, syn::AttrStyle::Inner(_))),
            );
            tokens.append_all(&self.items);
        });
    }
}

impl TryFrom<syn::ItemImpl> for ItemImpl {
    type Error = syn::Error;

    fn try_from(item_impl: syn::ItemImpl) -> Result<Self, Self::Error> {
        let impl_block_span = item_impl.span();
        if !Self::is_fvm_impl_block(&item_impl)? {
            return Err(format_err_spanned!(
                item_impl,
                "missing fvm annotations on implementation block or on any of its items"
            ));
        }
        if let Some(defaultness) = item_impl.defaultness {
            return Err(format_err_spanned!(
                defaultness,
                "default implementations are unsupported for fvm implementation blocks",
            ));
        }
        if let Some(unsafety) = item_impl.unsafety {
            return Err(format_err_spanned!(
                unsafety,
                "unsafe fvm implementation blocks are not supported",
            ));
        }
        if !item_impl.generics.params.is_empty() {
            return Err(format_err_spanned!(
                item_impl.generics.params,
                "generic fvm implementation blocks are not supported",
            ));
        }
        let impl_items = item_impl
            .items
            .into_iter()
            .map(<impl_item::ImplItem as TryFrom<_>>::try_from)
            .collect::<Result<Vec<_>, syn::Error>>()?;
        let is_trait_impl = item_impl.trait_.is_some();
        let (fvm_attrs, other_attrs) = attrs::partition_attributes(item_impl.attrs)?;
        let mut namespace: Option<attrs::Namespace> = None;
        if !fvm_attrs.is_empty() {
            let normalized =
                attrs::FvmAttribute::from_expanded(fvm_attrs).map_err(|err| {
                    err.into_combine(format_err!(impl_block_span, "at this invocation",))
                })?;
            normalized.ensure_no_conflicts(|arg| {
                match arg.kind() {
                    attrs::AttributeArg::Implementation | attrs::AttributeArg::Namespace(_) => {
                        Ok(())
                    }
                    _ => Err(None),
                }
            })?;
            namespace = normalized.namespace();
        }
        if namespace.is_some() && is_trait_impl {
            return Err(format_err!(
                impl_block_span,
                "namespace fvm property is not allowed on fvm trait implementation blocks",
            ));
        }
        Ok(Self {
            attrs: other_attrs,
            defaultness: item_impl.defaultness,
            unsafety: item_impl.unsafety,
            impl_token: item_impl.impl_token,
            generics: item_impl.generics,
            trait_: item_impl.trait_,
            self_ty: item_impl.self_ty,
            brace_token: item_impl.brace_token,
            items: impl_items,
            namespace,
        })
    }
}


impl ItemImpl {
    pub(super) fn is_fvm_impl_block(
        item_impl: &syn::ItemImpl,
    ) -> Result<bool, syn::Error> {

        if !attrs::contains_fvm_attributes(&item_impl.attrs)
            && item_impl
            .items
            .iter()
            .all(|item| !attrs::contains_fvm_attributes(item.attrs()))
        {
            return Ok(false);
        }

        let (fvm_attrs, _) = attrs::partition_attributes(item_impl.attrs.clone())?;
        let impl_block_span = item_impl.span();
        if !fvm_attrs.is_empty() {
            let normalized =
                attrs::FvmAttribute::from_expanded(fvm_attrs).map_err(|err| {
                    err.into_combine(format_err!(impl_block_span, "at this invocation",))
                })?;
            if normalized
                .ensure_first(&attrs::AttributeArgKind::Implementation)
                .is_ok()
            {
                return Ok(true);
            }
        }

        'repeat: for item in &item_impl.items {
            match item {
                syn::ImplItem::Method(method_item) => {
                    if !attrs::contains_fvm_attributes(&method_item.attrs) {
                        continue 'repeat;
                    }
                    let attr = attrs::first_fvm_attribute(&method_item.attrs)?
                        .expect("missing expected fvm attribute for struct");
                    match attr.first().kind() {
                        attrs::AttributeArg::Constructor | attrs::AttributeArg::Message => {
                            return Ok(true);
                        }
                        _ => continue 'repeat,
                    }
                }
                _ => continue 'repeat,
            }
        }
        Ok(false)
    }
}

impl ItemImpl {
    pub fn attrs(&self) -> &[syn::Attribute] {
        &self.attrs
    }

    pub fn self_type(&self) -> &syn::Type {
        self.self_ty.as_ref()
    }

    pub fn trait_path(&self) -> Option<&syn::Path> {
        self.trait_.as_ref().map(|(_, path, _)| path)
    }

    pub fn namespace(&self) -> Option<&attrs::Namespace> {
        self.namespace.as_ref()
    }

    pub fn iter_messages(&self) -> IterMessages {
        IterMessages::new(self)
    }

    pub fn iter_constructors(&self) -> IterConstructors {
        IterConstructors::new(self)
    }

    pub fn items(&self) -> &[impl_item::ImplItem] {
        &self.items
    }
}