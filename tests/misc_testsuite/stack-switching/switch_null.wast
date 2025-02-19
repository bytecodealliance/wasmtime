;;! stack_switching = true
(module
  (type $ft0 (func))
  (type $ct0 (cont $ft0))

  (type $ft1 (func (param (ref $ct0))))
  (type $ct1 (cont $ft1))

  (tag $t)

  (func $entry (export "entry")
    (switch $ct1 $t (ref.null $ct1))
  )
)

(assert_trap (invoke "entry") "null reference")
