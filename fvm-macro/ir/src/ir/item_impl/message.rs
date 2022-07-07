use crate::ir::{
    item_impl::{
        callable,
        callable::Callable,
    },
    attrs,
    attrs::SelectorOrWildcard,
    selector,
    utils,
};
use proc_macro2::{
    Ident,
    Span,
};
use syn::spanned::Spanned as _;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Receiver {
    Ref,
    RefMut,
}

impl quote::ToTokens for Receiver {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let receiver = match self {
            Self::Ref => quote::quote! { &self },
            Self::RefMut => quote::quote! { &mut self },
        };
        tokens.extend(receiver);
    }
}

impl Receiver {
    pub fn is_ref(self) -> bool {
        matches!(self, Self::Ref)
    }

    pub fn is_ref_mut(self) -> bool {
        matches!(self, Self::RefMut)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub(super) item: syn::ImplItemMethod,
    is_payable: bool,
    selector: Option<SelectorOrWildcard>,
}

impl quote::ToTokens for Message {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        self.item.to_tokens(tokens)
    }
}

impl Message {
    fn sanitize_attributes(
        method_item: &syn::ImplItemMethod,
    ) -> Result<(attrs::FvmAttribute, Vec<syn::Attribute>), syn::Error> {
        attrs::sanitize_attributes(
            method_item.span(),
            method_item.attrs.clone(),
            &attrs::AttributeArgKind::Message,
            |arg| {
                match arg.kind() {
                    attrs::AttributeArg::Message
                    | attrs::AttributeArg::Payable
                    | attrs::AttributeArg::Selector(_) => Ok(()),
                    _ => Err(None),
                }
            },
        )
    }
}

impl TryFrom<syn::ImplItemMethod> for Message {
    type Error = syn::Error;

    fn try_from(method_item: syn::ImplItemMethod) -> Result<Self, Self::Error> {
        let (fvm_attrs, other_attrs) = Self::sanitize_attributes(&method_item)?;
        let is_payable = fvm_attrs.is_payable();
        let selector = fvm_attrs.selector();
        Ok(Self {
            is_payable,
            selector,
            item: syn::ImplItemMethod {
                attrs: other_attrs,
                ..method_item
            },
        })
    }
}

impl callable::Callable for Message {
    fn kind(&self) -> callable::CallableKind {
        callable::CallableKind::Message
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
            _ => unreachable!("encountered invalid visibility for fvm message"),
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

impl Message {
    pub fn attrs(&self) -> &[syn::Attribute] {
        &self.item.attrs
    }

    pub fn receiver(&self) -> Receiver {
        match self.item.sig.inputs.iter().next() {
            Some(syn::FnArg::Receiver(receiver)) => {
                debug_assert!(receiver.reference.is_some());
                if receiver.mutability.is_some() {
                    Receiver::RefMut
                } else {
                    Receiver::Ref
                }
            }
            _ => unreachable!("encountered invalid receiver argument for fvm message"),
        }
    }

    pub fn output(&self) -> Option<&syn::Type> {
        match &self.item.sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, return_type) => Some(return_type),
        }
    }

    pub fn local_id(&self) -> u32 {
        utils::local_message_id(self.ident())
    }
}
