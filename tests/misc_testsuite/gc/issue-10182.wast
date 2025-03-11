;;! gc = true

(module
  (type (array (mut anyref)))

  (global (ref 0) (array.new_fixed 0 1 (array.new_fixed 0 0)))

  (func (export "")
    (local $l (ref 0))

    global.get 0
    local.set $l

    local.get 0
    i32.const 0
    local.get 0
    i32.const 0
    i32.const 1
    array.copy 0 0
  )
)

(assert_return (invoke ""))
