;;! target = "x86_64"

;; Test basic code generation for i32 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i32.load16_u") (param i32) (result i32)
    local.get 0
    i32.load16_u))

;; function u0:0(i32, i64 vmctx) -> i32 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i32, v1: i64):
;; @0032                               v3 = uextend.i64 v0
;; @0032                               v4 = global_value.i64 gv1
;; @0032                               v5 = iadd v4, v3
;; @0032                               v6 = uload16.i32 little heap v5
;; @0035                               jump block1(v6)
;;
;;                                 block1(v2: i32):
;; @0035                               return v2
;; }
