;;! target = "x86_64"

;; Test basic code generation for i32 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i32.load8_u") (param i32) (result i32)
    local.get 0
    i32.load8_u))

;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0031                               v4 = uextend.i64 v2
;; @0031                               v5 = global_value.i64 gv5
;; @0031                               v6 = iadd v5, v4
;; @0031                               v7 = uload8.i32 little heap v6
;; @0034                               jump block1(v7)
;;
;;                                 block1(v3: i32):
;; @0034                               return v3
;; }
