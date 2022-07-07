pub mod state;
pub mod event;


use core::result::Result;
use either::Either;
use itertools::Itertools;
use crate::ir::{
    attrs,
    attrs::{
        Attribute,
        FvmAttribute,
    },
    item_impl,
    item};


#[derive(Debug, PartialEq, Eq)]
pub enum Item {
    Fvm(FvmItem),
    Rust(syn::Item),
}

impl Item {
    pub fn map_fvm_item(&self) -> Option<&FvmItem> {
        match self {
            Item::Fvm(fvm_item) => Some(fvm_item),
            _ => None,
        }
    }

    pub fn map_rust_item(&self) -> Option<&syn::Item> {
        match self {
            Item::Rust(rust_item) => Some(rust_item),
            _ => None,
        }
    }
}

impl quote::ToTokens for Item {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Fvm(fvm_item) => fvm_item.to_tokens(tokens),
            Self::Rust(rust_item) => rust_item.to_tokens(tokens),
        }
    }
}

impl TryFrom<syn::Item> for Item {
    type Error = syn::Error;

    fn try_from(item: syn::Item) -> Result<Self, Self::Error> {
        // eprintln!("=========try_from=111=={:#?}", item);

        match item {
            syn::Item::Struct(item_struct) => {
                let attr = attrs::first_fvm_attribute(&item_struct.attrs)?.expect("missing expected fvm_macro attribute for struct");

                match attr.first().kind() {
                    attrs::AttributeArg::State => {
                        let value = <state::State as TryFrom<_>>::try_from(item_struct.clone());
                        let value = value
                            .map(Into::into)
                            .map(Self::Fvm);
                        return value;
                    }

                    _ => {}
                }
                let (_, value2): (Vec<FvmAttribute>, Vec<syn::Attribute>) = item_struct.attrs
                    .into_iter()
                    .map(|attr| -> Result<Attribute, syn::Error> {
                        if attr.path.is_ident("fvm_macro") {
                            return Ok(Attribute::Fvm(<FvmAttribute as TryFrom<_>>::try_from(attr).unwrap()));
                        }
                        Ok(Attribute::Other(attr))
                    })
                    .collect::<Result<Vec<Attribute>, syn::Error>>().unwrap()
                    .into_iter()
                    .partition_map(|attr| {
                        match attr {
                            Attribute::Fvm(fvm) => Either::Left(fvm),
                            Attribute::Other(other) => Either::Right(other),
                        }
                    });


                let ok = syn::ItemStruct {
                    attrs: value2,
                    ..item_struct
                };

                Ok(Self::Rust(syn::Item::Struct(ok)))
            }
            syn::Item::Impl(item_impl) => {
                if !item_impl::ItemImpl::is_fvm_impl_block(&item_impl)? {
                    return Ok(Self::Rust(item_impl.into()));
                }
                <item_impl::ItemImpl as TryFrom<_>>::try_from(item_impl)
                    .map(Into::into)
                    .map(Self::Fvm)
            }
            item => {
                Ok(Self::Rust(item))
            }
        }
    }
}


#[derive(Debug, PartialEq, Eq)]
pub enum FvmItem {
    State(state::State),
    Event(event::Event),
    ImplBlock(item_impl::ItemImpl),

}

impl quote::ToTokens for FvmItem {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::State(state) => state.to_tokens(tokens),
            Self::Event(event) => event.to_tokens(tokens),
            Self::ImplBlock(impl_block) => impl_block.to_tokens(tokens),
        }
    }
}

impl FvmItem {
    pub fn filter_map_storage_item(&self) -> Option<&item::state::State> {
        match self {
            FvmItem::State(state) => Some(state),
            _ => None,
        }
    }

    pub fn filter_map_event_item(&self) -> Option<&event::Event> {
        match self {
            FvmItem::Event(event) => Some(event),
            _ => None,
        }
    }

    pub fn filter_map_impl_block(&self) -> Option<&item_impl::ItemImpl> {
        match self {
            FvmItem::ImplBlock(impl_block) => Some(impl_block),
            _ => None,
        }
    }
}


impl From<state::State> for FvmItem {
    fn from(state: state::State) -> Self {
        Self::State(state)
    }
}

impl From<event::Event> for FvmItem {
    fn from(event: event::Event) -> Self {
        Self::Event(event)
    }
}

impl From<item_impl::ItemImpl> for FvmItem {
    fn from(impl_block: item_impl::ItemImpl) -> Self {
        Self::ImplBlock(impl_block)
    }
}