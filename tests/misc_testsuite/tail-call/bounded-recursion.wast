;;! tail_call = true

(module
  (func $ordinary (export "ordinary") (param $count i64) (result i64)
    local.get $count
    i64.eqz
    if (result i64)
      i64.const 42
    else
      local.get $count
      i64.const 1
      i64.sub
      call $ordinary
    end)

  (func $tail (export "tail") (param $count i64) (result i64)
    local.get $count
    i64.eqz
    if (result i64)
      i64.const 42
    else
      local.get $count
      i64.const 1
      i64.sub
      return_call $tail
    end))

(assert_exhaustion (invoke "ordinary" (i64.const 1000000)) "call stack exhausted")
(assert_return (invoke "tail" (i64.const 1000000)) (i64.const 42))
