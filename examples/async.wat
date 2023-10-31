(module
  (import "host" "print" (func $print (param i32)))
  (func $fibonacci (param $n i32) (result i32)
    (if
      (i32.lt_s (local.get $n) (i32.const 2))
      (then (return (local.get $n)))
    )
    (i32.add
      (call $fibonacci (i32.sub (local.get $n) (i32.const 1)))
      (call $fibonacci (i32.sub (local.get $n) (i32.const 2)))
    )
  )
  (func $print_fibonacci (param $n i32)
    (call $fibonacci (local.get $n))
    (call $print)
  )
  (export "print_fibonacci" (func $print_fibonacci))
)
