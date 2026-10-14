;;! stack_switching = true
;;! function_references = true
;;! bulk_memory = true

;; Sourced from https://github.com/bytecodealliance/wasmtime/issues/14508
(module
  (type $ft (func))
  (type $ct (cont $ft))
  (tag $t)
  (global $k (mut (ref null $ct)) (ref.null $ct))

  (func $body (suspend $t) (suspend $t))
  (elem declare func $body)

  (func (export "first")
    (block $h (result (ref $ct))
      (resume $ct (on $t $h) (cont.new $ct (ref.func $body)))
      (unreachable))
    (global.set $k))

  (func (export "second")
    (block $h (result (ref $ct))
      (resume $ct (on $t $h) (global.get $k))
      (unreachable))
    (global.set $k))
)
(invoke "first")
(invoke "second")
