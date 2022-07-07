use crate::ir;
use ir::item_mod;
use proc_macro2::TokenStream;

#[derive(Debug)]
pub struct Contract {
    item: item_mod::ItemMod,
}

impl Contract {
    pub fn new(fvm_item: TokenStream) -> Result<Self, syn::Error> {
        let module = syn::parse2::<syn::ItemMod>(fvm_item).unwrap();
        let fvm_module = item_mod::ItemMod::try_from(module).unwrap();
        Ok(Self { item: fvm_module })
    }

    pub fn module(&self) -> &item_mod::ItemMod {
        &self.item
    }
}

