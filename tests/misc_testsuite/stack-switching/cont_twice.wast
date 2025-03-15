;;! stack_switching = true
(module
  (type $ft (func))
  (type $ct (cont $ft))

  (func $f)
  (func (export "resume_twice")
    (local $k (ref $ct))
    (local.set $k (cont.new $ct (ref.func $f)))
    (resume $ct (local.get $k))
    (resume $ct (local.get $k))
  )
  (elem declare func $f)
)

(assert_trap (invoke "resume_twice") "continuation already consumed")