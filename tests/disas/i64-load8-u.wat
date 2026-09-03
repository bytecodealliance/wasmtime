;;! target = "x86_64"

;; Test basic code generation for i64 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i64.load8_u") (param i32) (result i64)
    local.get 0
    i64.load8_u))

;; function u0:0(i64 vmctx, i64, i32) -> i64 tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0031                               v3 = uextend.i64 v2
;; @0031                               v4 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @0031                               v5 = iadd v4, v3
;; @0031                               v6 = uload8.i64 little region4 v5
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return v6
;; }
