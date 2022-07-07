use crate::ir::{
    item_impl,
    item,
    attrs,
};

use proc_macro2::{Ident};

use crate::format_err_spanned;

use syn::{
    token,
};

#[derive(Debug, PartialEq, Eq)]
pub struct ItemMod {
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    mod_token: token::Mod,
    ident: syn::Ident,
    brace: token::Brace,
    items: Vec<item::Item>,
}

impl TryFrom<syn::ItemMod> for ItemMod {
    type Error = syn::Error;

    fn try_from(module: syn::ItemMod) -> Result<Self, Self::Error> {
        let (brace, items) = match module.content {
            Some((brace, items)) => (brace, items),
            None => {
                return Err(format_err_spanned!(
                    module,
                    "out-of-line fvm modules are not supported, use `#[fvm,::contract] mod name {{ ... }}`",
                ));
            }
        };

        let (_, other_attrs) = attrs::partition_attributes(module.attrs)?;
        let items = items
            .into_iter()
            .map(<item::Item as TryFrom<syn::Item>>::try_from)
            .collect::<Result<Vec<_>, syn::Error>>()?;

        Ok(Self {
            attrs: other_attrs,
            vis: module.vis,
            mod_token: module.mod_token,
            ident: module.ident,
            brace,
            items,
        })
    }
}


impl ItemMod {
    pub fn ident(&self) -> &Ident {
        &self.ident
    }


    pub fn state(&self) -> &item::state::State {
        let mut iter = IterFvmItems::new(self)
            .filter_map(|fvm_item| -> Option<&item::state::State> {
                fvm_item.filter_map_storage_item()
            });


        let storage = iter
            .next()
            .expect("encountered fvm module without a storage struct");

        assert!(
            iter.next().is_none(),
            "encountered multiple storage structs in fvm module"
        );
        storage
    }

    pub fn items(&self) -> &[item::Item] {
        self.items.as_slice()
    }

    pub fn impls(&self) -> IterItemImpls {
        IterItemImpls::new(self)
    }

    pub fn events(&self) -> IterEvents {
        IterEvents::new(self)
    }

    pub fn attrs(&self) -> &[syn::Attribute] {
        &self.attrs
    }

    pub fn vis(&self) -> &syn::Visibility {
        &self.vis
    }
}


#[derive(Debug)]
pub struct IterFvmItems<'a> {
    items_iter: core::slice::Iter<'a, item::Item>,
}

impl<'a> IterFvmItems<'a> {
    fn new(fvm_module: &'a ItemMod) -> Self {
        Self {
            items_iter: fvm_module.items.iter(),
        }
    }
}

impl<'a> Iterator for IterFvmItems<'a> {
    type Item = &'a item::FvmItem;

    fn next(&mut self) -> Option<Self::Item> {
        'repeat: loop {
            match self.items_iter.next() {
                None => return None,
                Some(item) => {
                    if let Some(event) = item.map_fvm_item() {
                        return Some(event);
                    }
                    continue 'repeat;
                }
            }
        }
    }
}

pub struct IterEvents<'a> {
    items_iter: IterFvmItems<'a>,
}

impl<'a> IterEvents<'a> {
    fn new(fvm_module: &'a ItemMod) -> Self {
        Self {
            items_iter: IterFvmItems::new(fvm_module),
        }
    }
}

impl<'a> Iterator for IterEvents<'a> {
    type Item = &'a item::event::Event;

    fn next(&mut self) -> Option<Self::Item> {
        'repeat: loop {
            match self.items_iter.next() {
                None => return None,
                Some(fvm_item) => {
                    if let Some(event) = fvm_item.filter_map_event_item() {
                        return Some(event);
                    }
                    continue 'repeat;
                }
            }
        }
    }
}

pub struct IterItemImpls<'a> {
    items_iter: IterFvmItems<'a>,
}

impl<'a> IterItemImpls<'a> {
    fn new(fvm_module: &'a ItemMod) -> Self {
        Self {
            items_iter: IterFvmItems::new(fvm_module),
        }
    }
}

impl<'a> Iterator for IterItemImpls<'a> {
    type Item = &'a item_impl::ItemImpl;

    fn next(&mut self) -> Option<Self::Item> {
        'repeat: loop {
            match self.items_iter.next() {
                None => return None,
                Some(fvm_item) => {
                    if let Some(event) = fvm_item.filter_map_impl_block() {
                        return Some(event);
                    }
                    continue 'repeat;
                }
            }
        }
    }
}

