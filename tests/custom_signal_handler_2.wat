(module
  (import "other_module" "read" (func $other_module.read (result i32)))
  (func $run (export "run") (result i32)
      call $other_module.read)
)
