use crate::GenerateCode;
use derive_more::From;
use proc_macro2::{
    TokenStream as TokenStream2,
};

use quote::{
    format_ident,
    quote,
    quote_spanned,
};

use fvm_ir::ir::{
    contract,
    item_impl,
    item_impl::{
        callable::{
            Callable
        },
    },
};

use fvm_ir::ir::item_impl::{message};
use heck::ToLowerCamelCase as _;


use syn::spanned::Spanned as _;

#[derive(From)]
pub struct State<'a> {
    contract: &'a contract::Contract,
}

impl core::convert::AsRef<contract::Contract> for State<'_> {
    fn as_ref(&self) -> &contract::Contract {
        self.contract
    }
}

impl GenerateCode for State<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let storage_span = self.contract.module().state().span();
        let storage_struct = self.generate_storage_struct();
        quote_spanned!(storage_span =>
            #storage_struct
        )
    }
}

impl State<'_> {
    fn generate_storage_struct(&self) -> TokenStream2 {
        let storage = self.contract.module().state();
        let span = storage.span();
        let ident = storage.ident();
        let attrs = storage.attrs();
        let fields = storage.fields();

        let item_impls1 = self
            .contract
            .module()
            .impls()
            .map(|item_impl| self.generate_item_impl1(item_impl)).collect::<Vec<_>>();

        let constructor_index = 1 as u64;

        quote_spanned!( span =>
            #(#attrs)*
            #[derive(Serialize_tuple, Deserialize_tuple, Clone, Debug, Default)]
            pub struct #ident {
                #( #fields ),*
            }
            impl State for #ident {
               fn load() -> Self {
                      // First, load the current state root.
                      let root = match sdk::sself::root() {
                          Ok(root) => root,
                          Err(err) => abort!(USR_ILLEGAL_STATE, "failed to get root: {:?}", err),
                      };

                      // Load the actor state from the state tree.
                      match Blockstore.get_cbor::<Self>(&root) {
                          Ok(Some(state)) => state,
                          Ok(None) => abort!(USR_ILLEGAL_STATE, "state does not exist"),
                          Err(err) => abort!(USR_ILLEGAL_STATE, "failed to get state: {}", err),
                            }
                }

                fn save(&self) -> Cid {
                        let serialized = match to_vec(self) {
                            Ok(s) => s,
                            Err(err) => abort!(USR_SERIALIZATION, "failed to serialize state: {:?}", err),
                        };
                        let cid = match sdk::ipld::put(Code::Blake2b256.into(), 32, DAG_CBOR, serialized.as_slice())
                        {
                            Ok(cid) => cid,
                            Err(err) => abort!(USR_SERIALIZATION, "failed to store initial state: {:}", err),
                        };
                        if let Err(err) = sdk::sself::set_root(&cid) {
                            abort!(USR_ILLEGAL_STATE, "failed to set root ciid: {:}", err);
                        }
                        cid
                }
            }

            impl #ident {
                pub fn invoke(id: u32) -> u32 {
                    let _params = sdk::message::params_raw(id).unwrap().1;
                    let _params = RawBytes::new(_params);
                    let ret: Option<RawBytes> = match sdk::message::method_number() {
                        #constructor_index =>#ident::constructor(),
                        #( #item_impls1 )*
                            _ => abort!(USR_UNHANDLED_MESSAGE, "unrecognized method"),
                        };

                    match ret {
                        None => NO_DATA_BLOCK_ID,
                        Some(v) => match sdk::ipld::put_block(DAG_CBOR, v.bytes()) {
                        Ok(id) => id,
                            Err(err) => abort!(USR_SERIALIZATION, "failed to store return value: {}", err),
                        },
                    }
                }


                pub fn constructor() -> Option<RawBytes> {
                    use fvm_shared::ActorID;
                    use fvm_sdk as sdk;

                    const INIT_ACTOR_ADDR: ActorID = 1;
                    if sdk::message::caller() != INIT_ACTOR_ADDR {
                        abort!(USR_FORBIDDEN, "constructor invoked by non-init actor");
                    }

                    let state = <#ident>::default();
                    state.save();
                    None
                }


            }
        )
    }
}

impl State<'_> {
    fn generate_item_impl1(&self, item_impl: &item_impl::ItemImpl) -> TokenStream2 {
        let impl_block = match item_impl.trait_path() {
            Some(_) => Self::generate_trait_item_impl(item_impl),
            None => Self::generate_inherent_item_impl1(self.contract, item_impl),
        };

        quote! {
            #impl_block
        }
    }

    fn generate_inherent_item_impl1(contract: &contract::Contract, item_impl: &item_impl::ItemImpl) -> TokenStream2 {
        assert!(item_impl.trait_path().is_none());
        let messages = item_impl
            .iter_messages()
            .enumerate()
            .map(|(index, cws)| Self::generate_inherent_message1(index as u64, contract, cws.callable()));
        quote! {
                #( #messages )*
            }
    }

    fn generate_trait_message(message: &message::Message) -> TokenStream2 {
        let span = message.span();
        let attrs = message.attrs();
        let vis = message.visibility();
        let receiver = message.receiver();
        let ident = message.ident();
        let output_ident =
            format_ident!("{}Output", ident.to_string().to_lower_camel_case());
        let inputs = message.inputs();
        let output = message
            .output()
            .cloned()
            .unwrap_or_else(|| syn::parse_quote! { () });
        let statements = message.statements();
        quote_spanned!(span =>
            type #output_ident = #output;
            #( #attrs )*
            #vis fn #ident(#receiver #( , #inputs )* ) -> Self::#output_ident {
                #( #statements )*
            }
        )
    }


    fn generate_trait_item_impl(item_impl: &item_impl::ItemImpl) -> TokenStream2 {
        assert!(item_impl.trait_path().is_some());
        let span = item_impl.span();
        let attrs = item_impl.attrs();
        let messages = item_impl
            .iter_messages()
            .map(|cws| Self::generate_trait_message(cws.callable()));
        let trait_path = item_impl
            .trait_path()
            .expect("encountered missing trait path for trait impl block");
        let self_type = item_impl.self_type();
        quote_spanned!(span =>
            #( #attrs )*
            impl #trait_path for #self_type {
                #( #messages )*
            }
        )
    }

    fn generate_inherent_message1(index: u64, contract: &contract::Contract, message: &message::Message) -> TokenStream2 {
        let ident = message.ident();
        let inputs = message.inputs();
        let state_ident = contract.module().state().ident();
        let index = index + 2;
        if inputs.len() == 1 {
            quote! { #index => <#state_ident>::load().#ident(), }
        } else {
            quote! { #index => <#state_ident>::load().#ident(_params), }
        }
    }
}
