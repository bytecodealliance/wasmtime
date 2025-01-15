;;! target = "x86_64"

;; Test basic code generation for i64 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i64.store32") (param i32 i64)
    local.get 0
    local.get 1
    i64.store32))

;; function u0:0(i64 vmctx, i64, i32, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @0033                               v4 = uextend.i64 v2
;; @0033                               v5 = load.i64 notrap aligned readonly checked v0+96
;; @0033                               v6 = iadd v5, v4
;; @0033                               istore32 little heap v3, v6
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return
;; }
