;;! bulk_memory = true
;;! function_references = true
;;! stack_switching = true

;; Regression test for buggy module-local type index to declared type
;; index conversion.
(module
  ;; These two function types canonicalize to the same module-interned type.
  ;; As a result, subsequent TypeIndex and ModuleInternedTypeIndex values do
  ;; not have the same numeric value.
  (type $duplicate (func))
  (type $ft0 (func))

  (type $ct0 (cont $ft0))

  (type $ft1 (func (param (ref $ct0))))
  (type $ct1 (cont $ft1))

  (tag $t)

  (func $f
    (cont.new $ct1 (ref.func $g))
    (switch $ct1 $t))
  (elem declare func $f)

  (func $g (type $ft1))
  (elem declare func $g)

  (func (export "entry") (result i32)
    (cont.new $ct0 (ref.func $f))
    (resume $ct0 (on $t switch))
    (i32.const 0))
)

(assert_return (invoke "entry") (i32.const 0))
