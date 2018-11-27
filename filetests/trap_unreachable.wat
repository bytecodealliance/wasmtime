(module
  (func $foo
    (unreachable)
  )
  (func $main
    (call $foo)
  )
  (start $main)
)
