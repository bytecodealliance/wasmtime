;;! target = "x86_64"

(module
  (func $imported (import "env" "f") (param i32) (result i32))
  (func $local (result externref externref funcref funcref)
    global.get 0
    global.get 1
    global.get 2
    global.get 3)

  (global (export "externref-imported") externref (ref.null extern))
  (global (export "externref-local") externref (ref.null extern))
  (global (export "funcref-imported") funcref (ref.func $imported))
  (global (export "funcref-local") funcref (ref.func $local)))

;; function u0:0(i64 vmctx, i64) -> i32, i32, i64, i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v17 = iconst.i64 80
;; @008f                               v7 = iadd v0, v17  ; v17 = 80
;; @008f                               v8 = load.i32 notrap aligned v7
;;                                     v16 = iconst.i64 96
;; @0091                               v10 = iadd v0, v16  ; v16 = 96
;; @0091                               v11 = load.i32 notrap aligned v10
;; @0093                               v13 = load.i64 notrap aligned table v0+112
;; @0095                               v15 = load.i64 notrap aligned table v0+128
;; @0097                               jump block1
;;
;;                                 block1:
;; @0097                               return v8, v11, v13, v15
;; }
