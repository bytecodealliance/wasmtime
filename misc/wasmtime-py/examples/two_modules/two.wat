(module
  (import "one" "memory" (memory $memory 0))
  (import "one" "bar" (func $bar (result i32)))
  (export "ask" (func $foo))

  (func $foo (result i32)
    call $bar
    ;; Deference returned pointer to the value from imported memory
    i32.load
  )
)
