;;! stack_switching = true
;;! reference_types = true

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