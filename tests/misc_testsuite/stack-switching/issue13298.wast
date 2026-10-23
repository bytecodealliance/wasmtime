;;! bulk_memory = true
;;! exceptions = true
;;! function_references = true
;;! stack_switching = true

;; Regression test for
;; https://github.com/bytecodealliance/wasmtime/issues/13298
;;
;; Stack-switching segfault due to missing updates to store stack
;; extents during switches #13298

(module
  (type $ft (func))
  (tag $t (type $ft))
  (type $ct (cont $ft))

  (func $callee (suspend $t))
  (elem declare func $callee)

  (func (export "go")
    (local $k (ref null $ct))
    (local.set $k (cont.new $ct (ref.func $callee)))
    (block $h (result (ref null $ct))
      (resume $ct (on $t $h) (local.get $k))
      (unreachable)
    )
    (drop)
    (unreachable)
  )
)

(assert_trap (invoke "go") "unreachable")