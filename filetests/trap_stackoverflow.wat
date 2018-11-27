(module
  (func $foo
    (call $main)
  )
  (func $main
    (call $foo)
  )
  (start $main)
)
