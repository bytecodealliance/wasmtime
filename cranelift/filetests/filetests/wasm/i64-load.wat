;;! target = "x86_64"

;; Test basic code generation for i32 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i64.load") (param i32) (result i64)
    local.get 0
    i64.load))

;; function u0:0(i32, i64 vmctx) -> i64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i32, v1: i64):
;; @002e                               v3 = uextend.i64 v0
;; @002e                               v4 = global_value.i64 gv1
;; @002e                               v5 = iadd v4, v3
;; @002e                               v6 = load.i64 little heap v5
;; @0031                               jump block1(v6)
;;
;;                                 block1(v2: i64):
;; @0031                               return v2
;; }