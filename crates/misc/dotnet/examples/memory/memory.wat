(module
  (type $t0 (func (param i32 i32)))
  (import "" "log" (func $.log (type $t0)))
  (memory (export "mem") 1 2)
  (data (i32.const 0) "Hello World")
  (func $run
    i32.const 0
    i32.const 11
    call $.log
  )
  (export "run" (func $run))
)
