;;! target = "x86_64"

;; Test basic code generation for i64 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i64.store32") (param i32 i64)
    local.get 0
    local.get 1
    i64.store32))

;; function u0:0(i32, i64, i64 vmctx) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i32, v1: i64, v2: i64):
;; @0033                               v3 = uextend.i64 v0
;; @0033                               v4 = global_value.i64 gv1
;; @0033                               v5 = iadd v4, v3
;; @0033                               istore32 little heap v1, v5
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return
;; }