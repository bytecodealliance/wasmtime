;;! target = "x86_64"

;; Test basic code generation for i64 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i64.load8_u") (param i32) (result i64)
    local.get 0
    i64.load8_u))

;; function u0:0(i32, i64 vmctx) -> i64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i32, v1: i64):
;; @0031                               v3 = uextend.i64 v0
;; @0031                               v4 = global_value.i64 gv1
;; @0031                               v5 = iadd v4, v3
;; @0031                               v6 = uload8.i64 little heap v5
;; @0034                               jump block1(v6)
;;
;;                                 block1(v2: i64):
;; @0034                               return v2
;; }