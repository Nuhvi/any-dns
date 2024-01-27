
use std::{fmt::Debug, error::Error};
use dyn_clone::DynClone;

/**
 * Trait to implement to make AnyDns use a custom handler.
 * Important: Handler must be clonable so it can be used by multiple threads.
 */
pub trait CustomHandler: DynClone + Send {
    fn lookup(&self, query: &Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>>;
}

/**
 * Clonable handler holder
 */
pub struct HandlerHolder {
    pub func: Box<dyn CustomHandler>,
}

impl Clone for HandlerHolder {
    fn clone(&self) -> Self {
        Self { func: dyn_clone::clone_box(&*self.func) }
    }
}

impl Debug for HandlerHolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerHolder").field("func", &"HandlerHolder").finish()
    }
}

impl HandlerHolder {
    /**
     * Bootstrap a holder from a struct that implements the CustomHandler.
     */
    pub fn new(f: impl CustomHandler + 'static) -> Self {
        HandlerHolder { func: Box::new(f) }
    }

    pub fn call(&self, query: &Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
        self.func.lookup(query)
    }
}

#[derive(Clone)]
pub struct EmptyHandler {
}

impl EmptyHandler {
    pub fn new() -> Self {
        EmptyHandler{}
    }
}

impl CustomHandler for EmptyHandler {
    fn lookup(&self, query: &Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
        Err("Not implemented".into())
    }
}


#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::custom_handler::EmptyHandler;

    use super::{HandlerHolder, CustomHandler};

    struct ClonableStruct {
        value: String
    }

    impl Clone for ClonableStruct {
        fn clone(&self) -> Self {
        Self { value: format!("{} cloned", self.value.clone()) }
    }
    }

    #[derive(Clone)]
    pub struct TestHandler {
        value: ClonableStruct
    }

    impl TestHandler {
        pub fn new(value: &str) -> Self {
            TestHandler{value: ClonableStruct{value: value.to_string()}}
        }
    }
    
    impl CustomHandler for TestHandler {
        fn lookup(&self, query: &Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
            println!("value {}", self.value.value);
            Err("Not implemented".into())
        }
    }


    #[test]
    fn run_processor() {
        let mut test1 = TestHandler::new("test1");
        let holder1 = HandlerHolder::new(test1);
        let cloned = holder1.clone();
        let result = cloned.call(&vec![]);
        assert!(result.is_err());

    }
}
