use crate::GenerateCode;
use derive_more::From;
use heck::ToLowerCamelCase as _;
use proc_macro2::TokenStream as TokenStream2;
use quote::{
    format_ident,
    quote,
    quote_spanned,
    ToTokens,
};
use syn::spanned::Spanned as _;

use fvm_ir::ir::{
    contract,
    item_impl,
    item_impl::{
        callable::{
            Callable
        },
    },
};
use fvm_ir::ir::item_impl::{constructor, impl_item, message};

#[derive(From)]
pub struct ItemImpls<'a> {
    contract: &'a contract::Contract,
}

impl core::convert::AsRef<contract::Contract> for ItemImpls<'_> {
    fn as_ref(&self) -> &contract::Contract {
        self.contract
    }
}

impl GenerateCode for ItemImpls<'_> {
    fn generate_code(&self) -> TokenStream2 {
        let item_impls = self
            .contract
            .module()
            .impls()
            .map(|item_impl| self.generate_item_impl(item_impl));
        quote! {
                #( #item_impls )*
        }
    }
}

impl ItemImpls<'_> {
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

    fn generate_inherent_constructor(constructor: &constructor::Constructor) -> TokenStream2 {
        let span = constructor.span();
        let attrs = constructor.attrs();
        let vis = constructor.visibility();
        let ident = constructor.ident();
        let inputs = constructor.inputs();
        let statements = constructor.statements();
        quote_spanned!(span =>
            #( #attrs )*
            #vis fn #ident( #( #inputs ),* ) -> Self {
                #( #statements )*
            }
        )
    }

    fn generate_inherent_message(_contract: &contract::Contract, message: &message::Message) -> TokenStream2 {
        let span = message.span();
        let attrs = message.attrs();
        let vis = message.visibility();
        let receiver = message.receiver();
        let ident = message.ident();
        let inputs = message.inputs();
        let output_arrow = message.output().map(|_| quote! { -> });
        let output = message.output();
        let statements = message.statements();
        quote_spanned!(span =>
            #( #attrs )*
            #vis fn #ident(#receiver #( , #inputs )* ) #output_arrow #output {
                _ = #receiver.save();
                #( #statements )*;
            }
        )
    }

    fn generate_inherent_item_impl(contract: &contract::Contract, item_impl: &item_impl::ItemImpl) -> TokenStream2 {
        assert!(item_impl.trait_path().is_none());

        let span = item_impl.span();
        let attrs = item_impl.attrs();
        let messages = item_impl
            .iter_messages()
            .map(|cws| Self::generate_inherent_message(contract, cws.callable()));
        let constructors = item_impl
            .iter_constructors()
            .map(|cws| Self::generate_inherent_constructor(cws.callable()));
        let other_items = item_impl
            .items()
            .iter()
            .filter_map(impl_item::ImplItem::filter_map_other_item)
            .map(ToTokens::to_token_stream);
        let self_type = item_impl.self_type();

        quote_spanned!(span =>
            #( #attrs )*
            impl #self_type {
                #( #constructors )*
                #( #messages )*
                #( #other_items )*
            }
        )
    }

    fn generate_item_impl(&self, item_impl: &item_impl::ItemImpl) -> TokenStream2 {
        let impl_block = match item_impl.trait_path() {
            Some(_) => Self::generate_trait_item_impl(item_impl),
            None => Self::generate_inherent_item_impl(self.contract, item_impl),
        };

        quote! {
            #impl_block
        }
    }
}
