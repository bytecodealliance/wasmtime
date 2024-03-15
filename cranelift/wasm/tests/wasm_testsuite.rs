use cranelift_codegen::isa::{CallConv, TargetFrontendConfig};
use cranelift_wasm::{translate_module, DummyEnvironment};
use target_lexicon::PointerWidth;

#[test]
fn reachability_is_correct() {
    let tests = vec![
        (
            r#"
        (module (func (param i32)
         (loop
          (block
           local.get 0
           br_if 0
           br 1))))"#,
            vec![
                (true, true),  // Loop
                (true, true),  // Block
                (true, true),  // LocalGet
                (true, true),  // BrIf
                (true, false), // Br
                (false, true), // End
                (true, true),  // End
                (true, true),  // End
            ],
        ),
        (
            r#"
        (module (func (param i32)
         (loop
          (block
           br 1
           nop))))"#,
            vec![
                (true, true),   // Loop
                (true, true),   // Block
                (true, false),  // Br
                (false, false), // Nop
                (false, false), // Nop
                (false, false), // Nop
                (false, false), // End
            ],
        ),
        (
            r#"
        (module (func (param i32) (result i32)
          i32.const 1
          return
          i32.const 42))"#,
            vec![
                (true, true),   // I32Const
                (true, false),  // Return
                (false, false), // I32Const
                (false, false), // End
            ],
        ),
    ];

    for (wat, expected_reachability) in tests {
        println!("testing wat:\n{}", wat);
        let mut env = DummyEnvironment::new(TargetFrontendConfig {
            default_call_conv: CallConv::SystemV,
            pointer_width: PointerWidth::U64,
        });
        env.test_expected_reachability(expected_reachability);
        let data = wat::parse_str(wat).unwrap();
        translate_module(data.as_ref(), &mut env).unwrap();
    }
}
