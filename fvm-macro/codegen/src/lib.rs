pub mod generator;
pub mod traits;

use traits::{GenerateCode, GenerateCodeUsing};
use proc_macro2::TokenStream as TokenStream2;
use fvm_ir::ir::contract;
use generator::{contract as gen_contract};

pub trait CodeGenerator: Sized {
    /// The underlying generator generating the code.
    type Generator: From<Self> + GenerateCode;
}


impl<'a> CodeGenerator for &'a contract::Contract {
    type Generator = gen_contract::Contract<'a>;
}


pub fn generate_code<T>(entity: T) -> TokenStream2
    where
        T: CodeGenerator,
{
    <T as CodeGenerator>::Generator::from(entity).generate_code()
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
