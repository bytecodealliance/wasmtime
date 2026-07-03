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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 402653184 "PublicGlobal"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @008f                               v2 = iconst.i64 80
;; @008f                               v3 = iadd v0, v2  ; v2 = 80
;; @008f                               v4 = load.i32 notrap aligned region2 v3
;; @0091                               v5 = iconst.i64 96
;; @0091                               v6 = iadd v0, v5  ; v5 = 96
;; @0091                               v7 = load.i32 notrap aligned region2 v6
;; @0093                               v8 = load.i64 notrap aligned region2 v0+112
;; @0095                               v9 = load.i64 notrap aligned region2 v0+128
;; @0097                               jump block1
;;
;;                                 block1:
;; @0097                               return v4, v7, v8, v9
;; }
