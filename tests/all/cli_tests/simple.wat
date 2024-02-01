(module
    (func (export "simple") (param i32) (result i32)
        local.get 0
    )
    (func (export "get_f32") (result f32) f32.const 100)
    (func (export "get_f64") (result f64) f64.const 100)
    (func (export "echo_f32") (param f32) (result f32) local.get 0)
    (func (export "echo_f64") (param f64) (result f64) local.get 0)
)
