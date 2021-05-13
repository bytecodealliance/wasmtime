(module
    (func (export "simple") (param i32) (result i32)
        local.get 0
    )
    (func (export "get_f32") (result f32) f32.const 100)
    (func (export "get_f64") (result f64) f64.const 100)
)
