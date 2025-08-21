;;! exceptions = true
;;! tail_call = true

(module
  (tag $tag)
  (export "_start" (func $start))
  (func $start
    block
      try_table (catch $tag 0)
        call $f
      end
    end
  )
  (func $f
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    return_call $throw
  )
  (func $throw (param i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    throw $tag
  )
)

(invoke "_start")
