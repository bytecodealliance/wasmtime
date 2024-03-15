(module
  (func $main
    (v128.const i32x4 1 2 3 4)
    (v128.const i32x4 1 2 3 4)
    (call $add)
    drop
  )
  (func $add (param $a v128) (param $b v128) (result v128)
    (local.get $a)
    (local.get $b)
    (i32x4.add)
  )
  (start $main)
)
