;;! target = "x86_64"

(module
  (data $passive "this is a passive data segment")
  (memory 0)

  (func (export "init") (param i32 i32 i32)
    local.get 0 ;; dst
    local.get 1 ;; src
    local.get 2 ;; cnt
    memory.init $passive)

  (func (export "drop")
    data.drop $passive))

;; function u0:0(i64 vmctx, i64, i32, i32, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+64
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+56
;;     sig0 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:1 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @003d                               v6 = load.i64 notrap aligned v0+64
;; @003d                               v7 = uextend.i64 v2
;; @003d                               v8 = uextend.i64 v4
;; @003d                               v9 = iconst.i64 1
;; @003d                               v10 = imul v8, v9  ; v9 = 1
;; @003d                               v11 = iadd v7, v10
;; @003d                               v12 = icmp ugt v11, v6
;; @003d                               trapnz v12, heap_oob
;; @003d                               v13 = load.i64 notrap aligned readonly can_move v0+56
;; @003d                               v14 = uextend.i64 v2
;; @003d                               v15 = iconst.i64 1
;; @003d                               v16 = imul v14, v15  ; v15 = 1
;; @003d                               v17 = iadd v13, v16
;; @003d                               v18 = load.i32 notrap aligned v0+152
;; @003d                               v19 = uextend.i64 v18
;; @003d                               v20 = uextend.i64 v3
;; @003d                               v21 = uextend.i64 v4
;; @003d                               v22 = iconst.i64 1
;; @003d                               v23 = imul v21, v22  ; v22 = 1
;; @003d                               v24 = iadd v20, v23
;; @003d                               v25 = icmp ugt v24, v19
;; @003d                               trapnz v25, heap_oob
;; @003d                               v26 = load.i64 notrap aligned v0+144
;; @003d                               v27 = uextend.i64 v3
;; @003d                               v28 = iadd v26, v27
;; @003d                               v29 = uextend.i64 v4
;; @003d                               call fn0(v0, v17, v28, v29)
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return
;; }
;;
;; function u0:1(i64 vmctx, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0044                               v2 = iconst.i32 0
;; @0044                               store notrap aligned v2, v0+152  ; v2 = 0
;; @0047                               jump block1
;;
;;                                 block1:
;; @0047                               return
;; }
