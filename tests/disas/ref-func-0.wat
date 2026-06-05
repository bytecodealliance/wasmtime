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
;;     region0 = 1610612736 "PublicGlobal"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @008f                               v7 = iconst.i64 80
;; @008f                               v8 = iadd v0, v7  ; v7 = 80
;; @008f                               v9 = load.i32 notrap aligned v8
;; @0091                               v11 = iconst.i64 96
;; @0091                               v12 = iadd v0, v11  ; v11 = 96
;; @0091                               v13 = load.i32 notrap aligned v12
;; @0093                               v15 = load.i64 notrap aligned region0 v0+112
;; @0095                               v17 = load.i64 notrap aligned region0 v0+128
;; @0097                               jump block1
;;
;;                                 block1:
;; @0097                               return v9, v13, v15, v17
;; }
