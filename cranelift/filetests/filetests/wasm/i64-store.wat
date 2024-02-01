;;! target = "x86_64"

;; Test basic code generation for i32 memory WebAssembly instructions.

(module
  (memory 1)
  (func (export "i64.store") (param i32 i64)
    local.get 0
    local.get 1
    i64.store))

;; function u0:0(i32, i64, i64 vmctx) fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0
;;
;;                                 block0(v0: i32, v1: i64, v2: i64):
;; @0031                               v3 = uextend.i64 v0
;; @0031                               v4 = global_value.i64 gv1
;; @0031                               v5 = iadd v4, v3
;; @0031                               store little heap v1, v5
;; @0034                               jump block1
;;
;;                                 block1:
;; @0034                               return
;; }