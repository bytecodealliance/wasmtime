(module
  (type (;0;) (func (param i32 i32) (result f64)))
  (func $main (type 0) (param i32 i32) (result f64)
    f64.const 1.0
    local.get 0
    local.get 1
    if (param i32)  ;; label = @2
      i64.load16_s align=1
      drop
    else
      unreachable
    end)
  (table (;0;) 63 255 funcref)
  (memory (;0;) 13 16)
  (export "t1" (table 0))
  (export "m1" (memory 0))
  (export "main" (func $main))
  (export "memory" (memory 0)))
