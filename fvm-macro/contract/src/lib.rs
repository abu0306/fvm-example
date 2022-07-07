use fvm_ir::ir::contract::Contract;
use fvm_codegen::generate_code;

#[proc_macro_attribute]
pub fn contract(attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    generate(attr.into(), item.into())
}


fn generate(_: proc_macro2::TokenStream, input: proc_macro2::TokenStream) -> proc_macro::TokenStream {
    let contract = Contract::new(input.clone()).unwrap();
    let generator_code = generate_code(&contract);
    return generator_code.into();
}