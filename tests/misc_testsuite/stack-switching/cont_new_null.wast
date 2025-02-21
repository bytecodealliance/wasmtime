;;! stack_switching = true
(module
  (type $ft (func))
  (type $ct (cont $ft))

  (func $entry (export "entry")
    (cont.new $ct (ref.null $ft))
    (drop)
  )
)

(assert_trap (invoke "entry") "null reference")
