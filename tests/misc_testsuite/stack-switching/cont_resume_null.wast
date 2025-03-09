;;! stack_switching = true
(module
  (type $ft (func))
  (type $ct (cont $ft))

  (func $entry (export "entry")
    (resume $ct (ref.null $ct))
  )
)

(assert_trap (invoke "entry") "null reference")
