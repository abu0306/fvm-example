#[fvm_macro::contract]
pub mod hello_world {
    use fvm_macro::*;

    #[fvm_macro(state)]
    pub struct HelloWorld {
        pub count: u64,
    }

    impl HelloWorld {
        #[fvm_macro(message)]
        pub fn say_hello(&mut self) -> Option<RawBytes> {
            self.count = self.count + 1;
            let ret = to_vec(format!("Hello world #{}!", self.count).as_str());
            match ret {
                Ok(ret) => {
                    return Some(RawBytes::new(ret));
                }
                Err(err) => {
                    abort!(
                      USR_ILLEGAL_STATE,
                      "failed to serialize return value: {:?}",
                      err
                      );
                }
            }
        }

        pub fn say_hi(&mut self) -> Option<RawBytes> {
            self.count = self.count + 1;
            let ret = to_vec(format!("Hello world #{}!", self.count).as_str());
            match ret {
                Ok(ret) => Some(RawBytes::new(ret)),
                Err(err) => {
                    abort!(
                      USR_ILLEGAL_STATE,
                      "failed to serialize return value: {:?}",
                      err
                      );
                }
            }
        }
    }
}
