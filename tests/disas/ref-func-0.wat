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

;; function u0:1(i64 vmctx, i64) -> r64, r64, i64, i64 fast {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext) -> r64 system_v
;;     sig1 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext system_v
;;     sig2 = (i64 vmctx, i32 uext) -> i32 uext system_v
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @008f                               v6 = global_value.i64 gv3
;; @008f                               v7 = load.i64 notrap aligned readonly v6+56
;; @008f                               v8 = load.i64 notrap aligned readonly v7+216
;; @008f                               v9 = iconst.i32 0
;; @008f                               v10 = call_indirect sig0, v8(v6, v9)  ; v9 = 0
;; @0091                               v11 = global_value.i64 gv3
;; @0091                               v12 = load.i64 notrap aligned readonly v11+56
;; @0091                               v13 = load.i64 notrap aligned readonly v12+216
;; @0091                               v14 = iconst.i32 1
;; @0091                               v15 = call_indirect sig0, v13(v11, v14)  ; v14 = 1
;; @0093                               v16 = global_value.i64 gv3
;; @0093                               v17 = load.i64 notrap aligned table v16+144
;; @0095                               v18 = global_value.i64 gv3
;; @0095                               v19 = load.i64 notrap aligned table v18+160
;; @0097                               jump block1(v10, v15, v17, v19)
;;
;;                                 block1(v2: r64, v3: r64, v4: i64, v5: i64):
;; @0097                               return v2, v3, v4, v5
;; }
