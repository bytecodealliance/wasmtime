mod multi_constructor;

pub(crate) type ConstructorVec<T> = Vec<T>;

struct Context;

struct It {
    i: u32,
    limit: u32,
}

impl multi_constructor::ContextIter for It {
    type Context = Context;
    type Output = u32;
    fn next(&mut self, _ctx: &mut Self::Context) -> Option<u32> {
        if self.i >= self.limit {
            None
        } else {
            let i = self.i;
            self.i += 1;
            Some(i)
        }
    }
}

impl multi_constructor::Context for Context {
    type etor_C_iter = It;
    fn etor_C(&mut self, value: u32) -> Option<It> {
        Some(It { i: 0, limit: value })
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
