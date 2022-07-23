mod multi_extractor;

#[derive(Clone)]
pub enum A {
    B,
    C,
}

struct Context;
impl multi_extractor::Context for Context {
    fn e1_etor(&mut self, arg0: u32, i: &mut usize) -> Option<(A, u32)> {
        if *i >= 32 {
            None
        } else {
            let idx = *i;
            *i += 1;
            let a = if arg0 & (1u32 << idx) != 0 {
                A::B
            } else {
                A::C
            };
            Some((a, idx as u32))
        }
    }

    fn e2_etor(&mut self, arg0: u32, i: &mut usize) -> Option<(A, u32)> {
        self.e1_etor(arg0, i)
    }
}

fn main() {
    let mut ctx = Context;
    let x = multi_extractor::constructor_Rule(&mut ctx, 0xf0);
    let y = multi_extractor::constructor_Rule(&mut ctx, 0);
    println!("x = {:?} y = {:?}", x, y);
}
