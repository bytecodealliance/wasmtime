(module
  (type $t0 (func))
  (import "" "hello" (func $.hello (type $t0)))
  (func $run
    call $.hello
  )
  (export "run" (func $run))
)
