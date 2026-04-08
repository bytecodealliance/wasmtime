;;! stack_switching = true
;;! gc = true

(assert_invalid
  (module
    (type $f (func))
    (type $c (cont $f))
    (type $s (struct (field (ref null $c))))
    (func (export "run")
      (drop (struct.new_default $s))
    )
  )
  "Stack switching feature not compatible with GC")
