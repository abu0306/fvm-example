use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use derive_more::From;
use fvm_ir::ir::{contract, item};
use crate::{
    generator,
    GenerateCode,
    GenerateCodeUsing,
};

#[derive(From)]
pub struct Contract<'a> {
    contract: &'a contract::Contract,
}

impl core::convert::AsRef<contract::Contract> for Contract<'_> {
    fn as_ref(&self) -> &contract::Contract {
        self.contract
    }
}


impl GenerateCode for Contract<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let module = self.contract.module();
        let state_ident = self.contract.module().state().ident();
        let ident = module.ident();
        let attrs = module.attrs();
        let vis = module.vis();

        let state = self.generate_code_using::<generator::state::State>();
        let item_impls = self.generate_code_using::<generator::item_impls::ItemImpls>();
        let non_fvm_items = self
            .contract
            .module()
            .items()
            .iter()
            .filter_map(item::Item::map_rust_item);
        quote! {
            #( #attrs )*
            #vis mod #ident {
                #( #non_fvm_items )*
                #state
                #item_impls
            }

            #[no_mangle]
            pub fn invoke(id: u32) -> u32 {
                 crate::#ident::#state_ident::invoke(id)
            }
        }
    }
}
