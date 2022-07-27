mod multi_constructor;

pub(crate) type ConstructorVec<T> = Vec<T>;

struct Context;

impl multi_constructor::Context for Context {
    fn etor_C(&mut self, value: u32, index: &mut usize) -> Option<u32> {
        let i = *index as u32;
        if i > value {
            None
        } else {
            *index += 1;
            Some(i)
        }
    }

    fn ctor_B(&mut self, value: u32) -> Option<Vec<u32>> {
        Some((0..value).rev().collect())
    }
}

fn main() {
    let mut ctx = Context;
    let l1 = multi_constructor::constructor_A(&mut ctx, 10).unwrap();
    let l2 = multi_constructor::constructor_D(&mut ctx, 5).unwrap();
    println!("l1 = {:?} l2 = {:?}", l1, l2);
}
