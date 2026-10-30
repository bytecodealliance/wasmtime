;;! bulk_memory = true
;;! exceptions = true
;;! function_references = true
;;! stack_switching = true

;; Wasm running inside a continuation can call a host function.
(module
  (type $ft (func (result i32)))
  (type $ct (cont $ft))

  (import "spectest" "print_i32" (func $print_i32 (param i32)))

  (func $a (result i32)
    (call $print_i32 (i32.const 122))
    (i32.const 123))
  (elem declare func $a)

  (func (export "entry") (result i32)
    (resume $ct (cont.new $ct (ref.func $a))))
)

(assert_return (invoke "entry") (i32.const 123))
