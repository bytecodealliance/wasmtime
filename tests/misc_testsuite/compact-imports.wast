;;! compact_imports = true

(module
  (func (export "one") (result i32) i32.const 1)
  (func (export "two") (result i32) i32.const 2)
  (func (export "four") (result i32) i32.const 4)
  (func (export "five") (result i32) i32.const 5)
)
(register "provider")

(module
  (import "provider" "one" (func $one (result i32)))
  (import "provider"
    (item "two" (func $two (result i32)))
  )
  (import "provider"
    (item "four")
    (item "five")
    (func (result i32))
  )

  (func (export "sum") (result i32)
    call $one
    call $two
    i32.add
    call 2
    i32.add
    call 3
    i32.add)
)

(assert_return (invoke "sum") (i32.const 12))
