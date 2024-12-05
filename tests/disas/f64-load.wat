;;! target = "x86_64"

;; Test basic code generation for f64 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "f64.load") (param i32) (result f64)
    local.get 0
    f64.load))

;; function u0:0(i64 vmctx, i64, i32) -> f64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+104
;;     gv5 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @002e                               v4 = uextend.i64 v2
;; @002e                               v5 = global_value.i64 gv5
;; @002e                               v6 = iadd v5, v4
;; @002e                               v7 = load.f64 little heap v6
;; @0031                               jump block1(v7)
;;
;;                                 block1(v3: f64):
;; @0031                               return v3
;; }
