use super::blake2::blake2b_256;
use crate::literal::HexLiteral;
use proc_macro2::TokenStream as TokenStream2;
use std::marker::PhantomData;
use syn::spanned::Spanned as _;
use crate::format_err;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Selector {
    bytes: [u8; 4],
}

#[derive(Debug, Copy, Clone)]
pub struct TraitPrefix<'a> {
    namespace: Option<&'a syn::LitStr>,
    trait_ident: &'a syn::Ident,
}

impl<'a> TraitPrefix<'a> {
    pub fn new(trait_ident: &'a syn::Ident, namespace: Option<&'a syn::LitStr>) -> Self {
        Self {
            namespace,
            trait_ident,
        }
    }

    pub fn namespace_bytes(&self) -> Vec<u8> {
        self.namespace
            .map(|namespace| namespace.value().into_bytes())
            .unwrap_or_default()
    }

    pub fn trait_ident(&self) -> &'a syn::Ident {
        self.trait_ident
    }
}

impl Selector {
    pub fn compute(input: &[u8]) -> Self {
        let mut output = [0; 32];
        blake2b_256(input, &mut output);
        Self::from([output[0], output[1], output[2], output[3]])
    }

    pub fn compose<'a, T>(trait_prefix: T, fn_ident: &syn::Ident) -> Self
        where
            T: Into<Option<TraitPrefix<'a>>>,
    {
        let fn_ident = fn_ident.to_string().into_bytes();
        let input_bytes: Vec<u8> = match trait_prefix.into() {
            Some(trait_prefix) => {
                let namespace = trait_prefix.namespace_bytes();
                let trait_ident = trait_prefix.trait_ident().to_string().into_bytes();
                let separator = &b"::"[..];
                if namespace.is_empty() {
                    [&trait_ident[..], &fn_ident[..]].join(separator)
                } else {
                    [&namespace[..], &trait_ident[..], &fn_ident[..]].join(separator)
                }
            }
            None => fn_ident.to_vec(),
        };
        Self::compute(&input_bytes)
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        self.bytes
    }

    pub fn into_be_u32(self) -> u32 {
        u32::from_be_bytes(self.bytes)
    }

    pub fn hex_lits(self) -> [syn::LitInt; 4] {
        self.bytes.map(<u8 as HexLiteral>::hex_padded_suffixed)
    }
}

impl From<[u8; 4]> for Selector {
    fn from(bytes: [u8; 4]) -> Self {
        Self { bytes }
    }
}

pub enum SelectorId {}

pub enum SelectorBytes {}

#[derive(Debug)]
pub struct SelectorMacro<T> {
    selector: Selector,
    input: syn::Lit,
    _marker: PhantomData<fn() -> T>,
}

impl<T> SelectorMacro<T> {
    pub fn selector(&self) -> Selector {
        self.selector
    }

    pub fn input(&self) -> &syn::Lit {
        &self.input
    }
}

impl<T> TryFrom<TokenStream2> for SelectorMacro<T> {
    type Error = syn::Error;

    fn try_from(input: TokenStream2) -> Result<Self, Self::Error> {
        let input_span = input.span();
        let lit = syn::parse2::<syn::Lit>(input).map_err(|error| {
            format_err!(
                input_span,
                "expected string or byte string literal as input: {}",
                error
            )
        })?;
        let input_bytes = match lit {
            syn::Lit::Str(ref lit_str) => lit_str.value().into_bytes(),
            syn::Lit::ByteStr(ref byte_str) => byte_str.value(),
            invalid => {
                return Err(format_err!(
                    invalid.span(),
                    "expected string or byte string literal as input. found {:?}",
                    invalid,
                ));
            }
        };
        let selector = Selector::compute(&input_bytes);
        Ok(Self {
            selector,
            input: lit,
            _marker: PhantomData,
        })
    }
}


