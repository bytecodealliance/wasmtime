(module
  (import "wasi_unstable" "poll_oneoff"
    (func $poll_oneoff (param i32 i32 i32 i32) (result i32))
  )

  (memory (export "memory") 65536)

  (func (export "run") (result i32)
    i32.const 0
    i32.const 0
    ;;such a large buffer should exhaust hostcall fuel
    i32.const 200000000
    i32.const 0
    call $poll_oneoff
  )
)
