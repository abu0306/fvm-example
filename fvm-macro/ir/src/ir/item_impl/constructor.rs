use proc_macro2::{
    Ident,
    Span,
};
use syn::spanned::Spanned as _;
use crate::ir::{attrs, attrs::{
    SelectorOrWildcard
}, item_impl::{
    callable
}, selector};

#[derive(Debug, PartialEq, Eq)]
pub struct Constructor {
    pub(super) item: syn::ImplItemMethod,
    is_payable: bool,
    selector: Option<SelectorOrWildcard>,
}

impl quote::ToTokens for Constructor {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.item.to_tokens(tokens)
    }
}

impl Constructor {
    fn sanitize_attributes(
        method_item: &syn::ImplItemMethod,
    ) -> Result<(attrs::FvmAttribute, Vec<syn::Attribute>), syn::Error> {
        attrs::sanitize_attributes(
            method_item.span(),
            method_item.attrs.clone(),
            &attrs::AttributeArgKind::Constructor,
            |arg| {
                match arg.kind() {
                    attrs::AttributeArg::Constructor
                    | attrs::AttributeArg::Payable
                    | attrs::AttributeArg::Selector(_) => Ok(()),
                    _ => Err(None),
                }
            },
        )
    }
}

impl TryFrom<syn::ImplItemMethod> for Constructor {
    type Error = syn::Error;

    fn try_from(method_item: syn::ImplItemMethod) -> Result<Self, Self::Error> {
        let (fvm_attrs, other_attrs) = Self::sanitize_attributes(&method_item)?;
        let is_payable = fvm_attrs.is_payable();
        let selector = fvm_attrs.selector();
        Ok(Constructor {
            selector,
            is_payable,
            item: syn::ImplItemMethod {
                attrs: other_attrs,
                ..method_item
            },
        })
    }
}

impl callable::Callable for Constructor {
    fn kind(&self) -> callable::CallableKind {
        callable::CallableKind::Constructor
    }

    fn ident(&self) -> &Ident {
        &self.item.sig.ident
    }

    fn user_provided_selector(&self) -> Option<&selector::Selector> {
        if let Some(SelectorOrWildcard::UserProvided(selector)) = self.selector.as_ref() {
            return Some(selector);
        }
        None
    }

    fn has_wildcard_selector(&self) -> bool {
        if let Some(SelectorOrWildcard::Wildcard) = self.selector {
            return true;
        }
        false
    }

    fn is_payable(&self) -> bool {
        self.is_payable
    }

    fn visibility(&self) -> callable::Visibility {
        match &self.item.vis {
            syn::Visibility::Public(vis_public) => callable::Visibility::Public(vis_public.clone()),
            syn::Visibility::Inherited => callable::Visibility::Inherited,
            _ => unreachable!("encountered invalid visibility for fvm constructor"),
        }
    }

    fn inputs(&self) -> callable::InputsIter {
        callable::InputsIter::from(self)
    }

    fn inputs_span(&self) -> Span {
        self.item.sig.inputs.span()
    }

    fn statements(&self) -> &[syn::Stmt] {
        &self.item.block.stmts
    }
}

impl Constructor {
    pub fn attrs(&self) -> &[syn::Attribute] {
        &self.item.attrs
    }
}
