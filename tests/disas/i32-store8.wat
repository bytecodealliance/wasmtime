;;! target = "x86_64"

;; Test basic code generation for i32 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i32.store8") (param i32 i32)
    local.get 0
    local.get 1
    i32.store8))

;; function u0:0(i64 vmctx, i64, i32, i32) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly checked gv3+96
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0032                               v4 = uextend.i64 v2
;; @0032                               v5 = global_value.i64 gv4
;; @0032                               v6 = iadd v5, v4
;; @0032                               istore8 little heap v3, v6
;; @0035                               jump block1
;;
;;                                 block1:
;; @0035                               return
;; }
