(module
  (memory (;0;) 0 0)
  (func (export "oob")
    i32.const 42
    f32.load align=1
    return))
