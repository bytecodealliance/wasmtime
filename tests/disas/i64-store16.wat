;;! target = "x86_64"

;; Test basic code generation for i64 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i64.store16") (param i32 i64)
    local.get 0
    local.get 1
    i64.store16))

;; function u0:0(i64 vmctx, i64, i32, i64) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 215 ""
;;     region3 = 105 ""
;;     region4 = 171 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @0033                               v4 = uextend.i64 v2
;; @0033                               v5 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0033                               v6 = iadd v5, v4
;; @0033                               istore16 little region4 v3, v6
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return
;; }
