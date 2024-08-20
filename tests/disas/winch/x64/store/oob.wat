;;! target = "x86_64"
;;! test = "winch"
;;! flags = " -O static-memory-maximum-size=0"
(module
  (memory 1)
  (func (export "foo") (param $i i32)
    i32.const 0
    (local.get $i)
    i32.store8 offset=4294967295
  )
)

