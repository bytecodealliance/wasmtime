;;! stack_switching = true
;;! reference_types = true
;;! bulk_memory = true

(module
  (type $ft (func))
  (type $ct (cont $ft))

  (func $fn
    (unreachable))

  (func $run_fn (export "run_fn")
    (resume $ct (cont.new $ct (ref.func $fn))))

  (elem declare func $fn)
)
(assert_trap (invoke "run_fn") "unreachable")

;; Unbounded recursion inside a continuation exhausts the stack.
(module
  (type $ft (func (param i32)))
  (type $ct (cont $ft))

  (func (export "entry")
    (resume $ct (i32.const 1_000_000_000) (cont.new $ct (ref.func $overflow))))

  (func $overflow (param $i i32)
    (if (i32.eqz (local.get $i)) (then (return)))
    (call $overflow (i32.sub (local.get $i) (i32.const 1))))

  (elem declare func $overflow)
)
(assert_exhaustion (invoke "entry") "call stack exhausted")
