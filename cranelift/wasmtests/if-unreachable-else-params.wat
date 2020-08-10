(module
  (type (;0;) (func (param i32)))
  (func $main (type 0) (param i32)
    i32.const 35
    loop (param i32)  ;; label = @1
      local.get 0
      if (param i32)  ;; label = @2
        i64.load16_s align=1
        unreachable
        unreachable
        unreachable
        unreachable
        unreachable
        local.get 0
        unreachable
        unreachable
        i64.load8_u offset=11789
        unreachable
      else
        i32.popcnt
        local.set 0
        return
        unreachable
      end
      unreachable
      unreachable
      nop
      f32.lt
      i32.store8 offset=82
      unreachable
    end
    unreachable
    unreachable
    unreachable
    unreachable)
  (table (;0;) 63 255 funcref)
  (memory (;0;) 13 16)
  (export "t1" (table 0))
  (export "m1" (memory 0))
  (export "main" (func $main))
  (export "memory" (memory 0)))
