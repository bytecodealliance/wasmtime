mod and_pattern_bug;

struct Context {
    enabled: bool,
}

impl and_pattern_bug::Context for Context {
    fn enabled(&mut self, _val: u32) -> Option<()> {
        if self.enabled {
            Some(())
        } else {
            None
        }
    }
}

fn main() {
    let mut ctx = Context { enabled: false };
    assert_eq!(and_pattern_bug::constructor_test(&mut ctx, 0), Some(23), "enabled is disabled");

    ctx.enabled = true;
    assert_eq!(and_pattern_bug::constructor_test(&mut ctx, 0), Some(0), "enabled is enabled");
}
