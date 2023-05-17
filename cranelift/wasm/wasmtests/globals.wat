(module
  (global $x (mut i32) (i32.const 4))
  (memory 1)
  (func $main (local i32)
    (i32.store (i32.const 0) (get_global $x))
  )
  (start $main)
)
