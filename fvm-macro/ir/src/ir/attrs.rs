use crate::{
    format_err_spanned,
    format_err,
};
use proc_macro2::{Ident, Span};
use syn::spanned::Spanned;
use crate::ir::{attrs, selector};


pub trait Attrs {
    fn attrs(&self) -> &[syn::Attribute];
}

impl Attrs for syn::ImplItem {
    fn attrs(&self) -> &[syn::Attribute] {
        match self {
            syn::ImplItem::Const(item) => &item.attrs,
            syn::ImplItem::Method(item) => &item.attrs,
            syn::ImplItem::Type(item) => &item.attrs,
            syn::ImplItem::Macro(item) => &item.attrs,
            _ => &[],
        }
    }
}


#[derive(Debug)]
pub enum Attribute {
    Fvm(FvmAttribute),
    Other(syn::Attribute),
}

impl TryFrom<syn::Attribute> for Attribute {
    type Error = syn::Error;

    fn try_from(attr: syn::Attribute) -> Result<Self, Self::Error> {
        if attr.path.is_ident("fvm_macro") {
            return <FvmAttribute as TryFrom<_>>::try_from(attr).map(Into::into);
        }
        Ok(Attribute::Other(attr))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FvmAttribute {
    args: Vec<AttributeFrag>,
}

impl Spanned for FvmAttribute {
    fn span(&self) -> Span {
        self.args
            .iter()
            .map(|arg| arg.span())
            .fold(self.first().span(), |fst, snd| {
                fst.join(snd).unwrap_or_else(|| self.first().span())
            })
    }
}

impl FvmAttribute {
    pub fn ensure_no_conflicts<'a, P>(
        &'a self,
        mut is_conflicting: P,
    ) -> Result<(), syn::Error>
        where
            P: FnMut(&'a attrs::AttributeFrag) -> Result<(), Option<syn::Error>>,
    {
        let mut err: Option<syn::Error> = None;
        for arg in self.args() {
            if let Err(reason) = is_conflicting(arg) {
                let conflict_err = format_err!(
                    arg.span(),
                    "encountered conflicting fvm attribute argument",
                );
                match &mut err {
                    Some(err) => {
                        err.combine(conflict_err);
                    }
                    None => {
                        err = Some(conflict_err);
                    }
                }
                if let Some(reason) = reason {
                    err.as_mut()
                        .expect("must be `Some` at this point")
                        .combine(reason);
                }
            }
        }
        if let Some(err) = err {
            return Err(err);
        }
        Ok(())
    }
}


impl FvmAttribute {
    pub fn ensure_first(&self, expected: &AttributeArgKind) -> Result<(), syn::Error> {
        if &self.first().arg.kind() != expected {
            return Err(format_err!(
                self.span(),
                "unexpected first fvm attribute argument",
            ));
        }
        Ok(())
    }

    pub fn from_expanded<A>(attrs: A) -> Result<Self, syn::Error>
        where
            A: IntoIterator<Item=Self>,
    {
        let args = attrs
            .into_iter()
            .flat_map(|attr| attr.args)
            .collect::<Vec<_>>();
        Ok(Self { args })
    }

    pub fn first(&self) -> &AttributeFrag {
        self.args
            .first()
            .expect("encountered invalid empty fvm attribute list")
    }

    pub fn is_anonymous(&self) -> bool {
        self.args()
            .any(|arg| matches!(arg.kind(), AttributeArg::Anonymous))
    }

    pub fn args(&self) -> ::core::slice::Iter<AttributeFrag> {
        self.args.iter()
    }

    pub fn namespace(&self) -> Option<attrs::Namespace> {
        self.args().find_map(|arg| {
            if let attrs::AttributeArg::Namespace(namespace) = arg.kind() {
                return Some(namespace.clone());
            }
            None
        })
    }

    pub fn selector(&self) -> Option<SelectorOrWildcard> {
        self.args().find_map(|arg| {
            if let attrs::AttributeArg::Selector(selector) = arg.kind() {
                return Some(*selector);
            }
            None
        })
    }

    pub fn is_payable(&self) -> bool {
        self.args()
            .any(|arg| matches!(arg.kind(), AttributeArg::Payable))
    }
}


impl From<FvmAttribute> for Attribute {
    fn from(fvm_attribute: FvmAttribute) -> Self {
        Attribute::Fvm(fvm_attribute)
    }
}

impl TryFrom<syn::Attribute> for FvmAttribute {
    type Error = syn::Error;

    fn try_from(attr: syn::Attribute) -> Result<Self, Self::Error> {
        if !attr.path.is_ident("fvm_macro") {
            return Err(format_err_spanned!(attr, "unexpected non-fvm attribute"));
        }


        match attr.parse_meta().map_err(|_| {
            format_err_spanned!(attr, "unexpected fvm attribute structure")
        }).unwrap() {
            syn::Meta::List(meta_list) => {
                let args = meta_list
                    .nested
                    .into_iter()
                    .map(<AttributeFrag as TryFrom<_>>::try_from)
                    .collect::<Result<Vec<_>, syn::Error>>().unwrap();

                if args.is_empty() {
                    return Err(format_err_spanned!(
                        attr,
                        "encountered unsupported empty fvm attribute"
                    ));
                }

                Ok(FvmAttribute { args })
            }
            _ => Err(format_err_spanned!(attr, "unknown fvm attribute")),
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttributeFrag {
    pub ast: syn::Meta,
    pub arg: AttributeArg,
}


impl AttributeFrag {
    pub fn kind(&self) -> &AttributeArg {
        &self.arg
    }
}

impl Spanned for AttributeFrag {
    fn span(&self) -> Span {
        self.ast.span()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AttributeArg {
    Actor,
    State,
    Event,
    Anonymous,
    Topic,
    Message,
    Constructor,
    Payable,
    Implementation,
    Selector(SelectorOrWildcard),
    Namespace(Namespace),

}


impl TryFrom<syn::NestedMeta> for AttributeFrag {
    type Error = syn::Error;

    fn try_from(nested_meta: syn::NestedMeta) -> Result<Self, Self::Error> {
        match nested_meta {
            syn::NestedMeta::Meta(meta) => {
                match &meta {
                    //TODO:syn::Meta::NameValue
                    syn::Meta::NameValue(_) => {
                        Err(format_err_spanned!(
                            meta,
                            "unknown fvm attribute argument (list)"
                        ))
                    }
                    syn::Meta::Path(path) => {
                        path
                            .get_ident()
                            .map(Ident::to_string)
                            .ok_or_else(|| format_err_spanned!(meta, "unknown fvm attribute (path)"))
                            .and_then(|ident| match ident.as_str() {
                                "actor" => Ok(AttributeArg::Actor),
                                "state" => Ok(AttributeArg::State),
                                "message" => Ok(AttributeArg::Message),
                                "constructor" => Ok(AttributeArg::Constructor),
                                "event" => Ok(AttributeArg::Event),
                                "anonymous" => Ok(AttributeArg::Anonymous),
                                "topic" => Ok(AttributeArg::Topic),
                                "payable" => Ok(AttributeArg::Payable),
                                "impl" => Ok(AttributeArg::Implementation),
                                _ => Err(format_err_spanned!(
                                    meta, "unknown state attribute (path)"
                                ))
                            })
                            .map(|kind| AttributeFrag { ast: meta, arg: kind })
                    }
                    syn::Meta::List(_) => {
                        Err(format_err_spanned!(
                            meta,
                            "unknown state attribute argument (list)"
                        ))
                    }
                }
            }
            syn::NestedMeta::Lit(_) => {
                Err(format_err_spanned!(
                    nested_meta,
                    "unknown state attribute argument (literal)"
                ))
            }
        }
    }
}

impl AttributeArg {
    pub fn kind(&self) -> AttributeArgKind {
        match self {
            Self::State => AttributeArgKind::State,
            Self::Event => AttributeArgKind::Event,
            Self::Anonymous => AttributeArgKind::Anonymous,
            Self::Topic => AttributeArgKind::Topic,
            Self::Message => AttributeArgKind::Message,
            Self::Constructor => AttributeArgKind::Constructor,
            Self::Payable => AttributeArgKind::Payable,
            Self::Selector(_) => AttributeArgKind::Selector,
            Self::Namespace(_) => AttributeArgKind::Namespace,
            Self::Implementation => AttributeArgKind::Implementation,
            _ => AttributeArgKind::Actor,
        }
    }
}


pub fn contains_fvm_attributes<'a, I>(attrs: I) -> bool
    where
        I: IntoIterator<Item=&'a syn::Attribute>,
{
    attrs.into_iter().any(|attr| attr.path.is_ident("fvm_macro"))
}


pub fn first_fvm_attribute<'a, I>(
    attrs: I,
) -> Result<Option<attrs::FvmAttribute>, syn::Error>
    where
        I: IntoIterator<Item=&'a syn::Attribute>,
{
    let first = attrs.into_iter().find(|attr| attr.path.is_ident("fvm_macro"));
    match first {
        None => Ok(None),
        Some(fvm_attr) => FvmAttribute::try_from(fvm_attr.clone()).map(Some),
    }
}

pub fn sanitize_attributes<I, C>(
    _: Span,
    attrs: I,
    _: &AttributeArgKind,
    _: C,
) -> Result<(FvmAttribute, Vec<syn::Attribute>), syn::Error>
    where
        I: IntoIterator<Item=syn::Attribute>,
        C: FnMut(&AttributeFrag) -> Result<(), Option<syn::Error>>,
{
    let (fvm_attrs, other_attrs) = partition_attributes(attrs)?;
    let normalized = FvmAttribute::from_expanded(fvm_attrs).unwrap();
    Ok((normalized, other_attrs))
}

pub fn partition_attributes<I>(
    attrs: I,
) -> Result<(Vec<FvmAttribute>, Vec<syn::Attribute>), syn::Error>
    where
        I: IntoIterator<Item=syn::Attribute>,
{
    use either::Either;
    use itertools::Itertools as _;
    let (fvm_attrs, others) = attrs
        .into_iter()
        .map(<Attribute as TryFrom<_>>::try_from)
        .collect::<Result<Vec<Attribute>, syn::Error>>()?
        .into_iter()
        .partition_map(|attr| {
            match attr {
                Attribute::Fvm(fvm_attr) => Either::Left(fvm_attr),
                Attribute::Other(other_attr) => Either::Right(other_attr),
            }
        });


    Ok((fvm_attrs, others))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AttributeArgKind {
    Actor,
    State,
    Event,
    Anonymous,
    Topic,
    Message,
    Constructor,
    Payable,
    Selector,
    Extension,
    Namespace,
    Implementation,
    HandleStatus,
    ReturnsResult,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SelectorOrWildcard {
    Wildcard,
    UserProvided(selector::Selector),
}

impl core::fmt::Display for SelectorOrWildcard {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> Result<(), core::fmt::Error> {
        match self {
            Self::UserProvided(selector) => core::fmt::Debug::fmt(&selector, f),
            Self::Wildcard => write!(f, "_"),
        }
    }
}


#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Namespace {
    bytes: Vec<u8>,
}

impl From<Vec<u8>> for Namespace {
    fn from(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl Namespace {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}