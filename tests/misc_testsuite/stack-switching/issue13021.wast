;;! gc = true
;;! stack_switching = true

;; Regression test for issue https://github.com/bytecodealliance/wasmtime/issues/13021
(module
  (type $ft (func))
  (type $ct (cont $ft))
  (type $arr (array (mut (ref null $ct))))
  (func (export "boom")
    (local $a (ref $arr))
    (local.set $a (array.new_default $arr (i32.const 2)))
    (array.copy $arr $arr
      (local.get $a) (i32.const 0)
      (local.get $a) (i32.const 0)
      (i32.const 1))
  )
)
(assert_return (invoke "boom"))