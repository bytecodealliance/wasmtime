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
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @008f                               v6 = global_value.i64 gv3
;; @008f                               v7 = iconst.i32 0
;; @008f                               v8 = call fn0(v6, v7)  ; v7 = 0
;; @0091                               v9 = global_value.i64 gv3
;; @0091                               v10 = iconst.i32 1
;; @0091                               v11 = call fn0(v9, v10)  ; v10 = 1
;; @0093                               v12 = global_value.i64 gv3
;; @0093                               v13 = load.i64 notrap aligned table v12+144
;; @0095                               v14 = global_value.i64 gv3
;; @0095                               v15 = load.i64 notrap aligned table v14+160
;; @0097                               jump block1(v8, v11, v13, v15)
;;
;;                                 block1(v2: r64, v3: r64, v4: i64, v5: i64):
;; @0097                               return v2, v3, v4, v5
;; }
