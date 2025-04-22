;;! stack_switching = true
(module
  (type $ft (func))
  (type $ct (cont $ft))

  (func $entry (export "entry")
    (cont.bind $ct $ct (ref.null $ct))
    (drop)
  )
)

(assert_trap (invoke "entry") "null reference")
