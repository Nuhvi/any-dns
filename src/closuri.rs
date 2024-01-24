// #[derive(Debug, Clone)]
// struct ValHolder {
//     val: usize
// }

// #[derive(Clone)]
// struct FnHolder {
//     func: Box<dyn Fn(usize) -> usize>,
// }

// impl FnHolder {
//     pub fn new(f: impl Fn(usize) -> usize + 'static) -> Self {
//         FnHolder { func: Box::new(f) }
//     }

//     pub fn call(&self) -> usize {
//         (self.func)(1)
//     }
// }

// #[cfg(test)]
// mod tests {
//     use crate::closuri::ValHolder;

//     use super::FnHolder;


//     #[test]
//     fn run_processor() {
//         let val = ValHolder{
//             val: 6
//         };
//         let holder = FnHolder::new(move |a| a + val.val);

//         println!("res {}", holder.call());

//     }
// }
