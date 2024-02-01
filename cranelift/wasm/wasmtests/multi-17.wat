(module
  (func $main (type 0) (param i32 i32 i32) (result i32)
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0

    i32.const 0
    if (param i32 i32 i32) (result i32)  ;; label = @1
      br 0 (;@1;)
    else
      call $main
    end

    i32.const 0

    i32.const 0
    if (param i32 i32 i32) (result i32)  ;; label = @1
      drop
      drop
    else
      drop
      drop
    end
  )
  (export "main" (func $main)))
