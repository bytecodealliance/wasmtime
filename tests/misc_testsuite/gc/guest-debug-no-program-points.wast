;;! gc = true
;;! flags = "-D guest-debug"

;; Regression test for #14563. With guest debugging, some generated functions,
;; e.g. the one running the module's global initializers, have no program
;; points for a wasm PC. The large global initializers force GCs during
;; instantiation, and nothing should go awry as a result.
(module
  (type $a (array (mut i8)))
  (func (export "f")
    (local externref externref externref externref
           externref externref externref externref
           externref externref externref externref
           externref externref externref externref
           externref externref externref externref
           externref externref externref externref
           externref externref externref externref
           externref externref externref externref))
  (global (ref $a) (array.new_default $a (i32.const 100000)))
  (global (ref $a) (array.new_default $a (i32.const 100000)))
  (global (ref $a) (array.new_default $a (i32.const 100000))))
