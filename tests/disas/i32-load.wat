;;! target = "x86_64"

;; Test basic code generation for i32 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i32.load") (param i32) (result i32)
    local.get 0
    i32.load))

;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned gv0+72
;;     gv2 = load.i64 notrap aligned readonly can_move checked gv0+64
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002e                               v4 = uextend.i64 v2
;; @002e                               v5 = load.i64 notrap aligned readonly can_move checked v0+64
;; @002e                               v6 = iadd v5, v4
;; @002e                               v7 = load.i32 little heap v6
;; @0031                               jump block1
;;
;;                                 block1:
;; @0031                               return v7
;; }
