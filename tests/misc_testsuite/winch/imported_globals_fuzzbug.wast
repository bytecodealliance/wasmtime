
(module $a
  (global (export "b") i32 (i32.const 0))
)
(register "a")

(module $index
  (import "a" "b" (global i32))
  (func (export "start")
    (local i32 i32 i32)
    local.get 2
    local.get 2
    local.get 2
    local.get 2
    local.get 2
    local.get 2
    local.get 2
    local.get 2
    global.get 0
    global.get 0
    global.get 0
    global.get 0
    global.get 0
    global.get 0
    global.get 0
    global.get 0
    global.get 0
    global.get 0
    global.get 0
    global.get 0
    local.get 2
    global.get 0
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
    drop
  )
)

(assert_return (invoke "start"))
