fn main() {
    let a = 3;
    foo::bar(a);
}

mod foo {
    pub fn bar(x: u32) -> u32 {
        let mut sum = 0;
        for i in 0..x {
            sum += i;
        }
        sum
    }
}
