use crate::ir::{
    selector,
    item_impl,
    item_impl::{
        constructor, message,
    },
};

use core::fmt;
use proc_macro2::{
    Ident,
    Span,
};
use quote::ToTokens as _;
use syn::spanned::Spanned as _;

#[derive(Debug, Copy, Clone)]
pub enum CallableKind {
    Message,
    Constructor,
}

impl fmt::Display for CallableKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message => write!(f, "message"),
            Self::Constructor => write!(f, "constructor"),
        }
    }
}

/// Wrapper for a callable that adds its composed selector.
#[derive(Debug)]
pub struct CallableWithSelector<'a, C> {
    composed_selector: selector::Selector,
    /// The parent implementation block.
    item_impl: &'a item_impl::ItemImpl,
    callable: &'a C,
}

impl<C> Copy for CallableWithSelector<'_, C> {}

impl<C> Clone for CallableWithSelector<'_, C> {
    fn clone(&self) -> Self {
        Self {
            composed_selector: self.composed_selector,
            item_impl: self.item_impl,
            callable: self.callable,
        }
    }
}

impl<'a, C> CallableWithSelector<'a, C>
    where
        C: Callable,
{
    pub(super) fn new(item_impl: &'a item_impl::ItemImpl, callable: &'a C) -> Self {
        Self {
            composed_selector: compose_selector(item_impl, callable),
            item_impl,
            callable,
        }
    }
}

impl<'a, C> CallableWithSelector<'a, C> {
    pub fn composed_selector(&self) -> selector::Selector {
        self.composed_selector
    }

    pub fn callable(&self) -> &'a C {
        self.callable
    }

    pub fn item_impl(&self) -> &'a item_impl::ItemImpl {
        self.item_impl
    }
}

impl<'a, C> Callable for CallableWithSelector<'a, C>
    where
        C: Callable,
{
    fn kind(&self) -> CallableKind {
        <C as Callable>::kind(self.callable)
    }

    fn ident(&self) -> &Ident {
        <C as Callable>::ident(self.callable)
    }

    fn user_provided_selector(&self) -> Option<&selector::Selector> {
        <C as Callable>::user_provided_selector(self.callable)
    }

    fn is_payable(&self) -> bool {
        <C as Callable>::is_payable(self.callable)
    }

    fn has_wildcard_selector(&self) -> bool {
        <C as Callable>::has_wildcard_selector(self.callable)
    }

    fn visibility(&self) -> Visibility {
        <C as Callable>::visibility(self.callable)
    }

    fn inputs(&self) -> InputsIter {
        <C as Callable>::inputs(self.callable)
    }

    fn inputs_span(&self) -> Span {
        <C as Callable>::inputs_span(self.callable)
    }

    fn statements(&self) -> &[syn::Stmt] {
        <C as Callable>::statements(self.callable)
    }
}

impl<'a, C> ::core::ops::Deref for CallableWithSelector<'a, C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        self.callable
    }
}

pub trait Callable {
    fn kind(&self) -> CallableKind;

    fn ident(&self) -> &Ident;

    fn user_provided_selector(&self) -> Option<&selector::Selector>;

    fn is_payable(&self) -> bool;

    fn has_wildcard_selector(&self) -> bool;

    fn visibility(&self) -> Visibility;

    fn inputs(&self) -> InputsIter;

    fn inputs_span(&self) -> Span;

    fn statements(&self) -> &[syn::Stmt];
}

pub fn compose_selector<C>(item_impl: &item_impl::ItemImpl, callable: &C) -> selector::Selector
    where
        C: Callable,
{
    if let Some(selector) = callable.user_provided_selector() {
        return *selector;
    }
    let callable_ident = callable.ident().to_string().into_bytes();
    let namespace_bytes = item_impl
        .namespace()
        .map(|namespace| namespace.as_bytes().to_vec())
        .unwrap_or_default();
    let separator = &b"::"[..];
    let joined = match item_impl.trait_path() {
        None => {
            // Inherent implementation block:
            if namespace_bytes.is_empty() {
                callable_ident
            } else {
                [namespace_bytes, callable_ident].join(separator)
            }
        }
        Some(path) => {
            // Trait implementation block:
            //
            // We need to separate between full-path, e.g. `::my::full::Path`
            // starting with `::` and relative paths for the composition.
            let path_bytes = if path.leading_colon.is_some() {
                let mut str_repr = path.to_token_stream().to_string();
                str_repr.retain(|c| !c.is_whitespace());
                str_repr.into_bytes()
            } else {
                path.segments
                    .last()
                    .expect("encountered empty trait path")
                    .ident
                    .to_string()
                    .into_bytes()
            };
            if namespace_bytes.is_empty() {
                [path_bytes, callable_ident].join(separator)
            } else {
                [namespace_bytes, path_bytes, callable_ident].join(separator)
            }
        }
    };
    selector::Selector::compute(&joined)
}


#[derive(Debug, Clone)]
pub enum Visibility {
    Public(syn::VisPublic),
    Inherited,
}

impl quote::ToTokens for Visibility {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Public(vis_public) => vis_public.to_tokens(tokens),
            Self::Inherited => (),
        }
    }
}

impl Visibility {
    pub fn is_pub(&self) -> bool {
        matches!(self, Self::Public(_))
    }

    pub fn is_inherited(&self) -> bool {
        matches!(self, Self::Inherited)
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Public(vis_public) => Some(vis_public.span()),
            Self::Inherited => None,
        }
    }
}


pub struct InputsIter<'a> {
    iter: syn::punctuated::Iter<'a, syn::FnArg>,
}

impl<'a> InputsIter<'a> {
    pub(crate) fn new<P>(inputs: &'a syn::punctuated::Punctuated<syn::FnArg, P>) -> Self {
        Self {
            iter: inputs.iter(),
        }
    }
}

impl<'a> From<&'a message::Message> for InputsIter<'a> {
    fn from(message: &'a message::Message) -> Self {
        Self::new(&message.item.sig.inputs)
    }
}

impl<'a> From<&'a constructor::Constructor> for InputsIter<'a> {
    fn from(constructor: &'a constructor::Constructor) -> Self {
        Self::new(&constructor.item.sig.inputs)
    }
}

impl<'a> Iterator for InputsIter<'a> {
    type Item = &'a syn::PatType;

    fn next(&mut self) -> Option<Self::Item> {
        'repeat: loop {
            match self.iter.next() {
                None => return None,
                Some(syn::FnArg::Typed(pat_typed)) => return Some(pat_typed),
                Some(syn::FnArg::Receiver(_)) => continue 'repeat,
            }
        }
    }
}

impl<'a> ExactSizeIterator for InputsIter<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
