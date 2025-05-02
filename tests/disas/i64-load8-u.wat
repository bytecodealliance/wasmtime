;;! target = "x86_64"

;; Test basic code generation for i64 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i64.load8_u") (param i32) (result i64)
    local.get 0
    i64.load8_u))

;; function u0:0(i64 vmctx, i64, i32) -> i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned gv0+72
;;     gv2 = load.i64 notrap aligned readonly can_move checked gv0+64
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0031                               v4 = uextend.i64 v2
;; @0031                               v5 = load.i64 notrap aligned readonly can_move checked v0+64
;; @0031                               v6 = iadd v5, v4
;; @0031                               v7 = uload8.i64 little heap v6
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return v7
;; }
