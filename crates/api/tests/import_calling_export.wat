(module
  (type $t0 (func))
  (import "" "imp" (func $.imp (type $t0)))
  (func $run call $.imp)
  (func $other)
  (export "run" (func $run))
  (export "other" (func $other))
)