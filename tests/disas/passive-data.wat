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
;;     fn0 = colocated u805306368:4 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @003d                               v6 = load.i64 notrap aligned v0+64
;; @003d                               v7 = uextend.i64 v2
;; @003d                               v8 = uextend.i64 v4
;; @003d                               v9 = iadd v7, v8
;; @003d                               v10 = icmp ule v9, v6
;; @003d                               trapz v10, heap_oob
;; @003d                               v11 = uextend.i64 v2
;; @003d                               v12 = load.i64 notrap aligned readonly can_move v0+56
;; @003d                               v13 = iadd v12, v11
;; @003d                               v15 = uload32 notrap aligned v0+152
;; @003d                               v16 = uextend.i64 v3
;; @003d                               v17 = uextend.i64 v4
;; @003d                               v18 = iadd v16, v17
;; @003d                               v19 = icmp ugt v18, v15
;; @003d                               trapnz v19, heap_oob
;; @003d                               v20 = load.i64 notrap aligned v0+144
;; @003d                               v21 = uextend.i64 v3
;; @003d                               v22 = iadd v20, v21
;; @003d                               v24 = uextend.i64 v4
;; @003d                               call fn0(v0, v13, v22, v24)
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
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0044                               v3 = iconst.i32 0
;; @0044                               store notrap aligned v3, v0+152  ; v3 = 0
;; @0047                               jump block1
;;
;;                                 block1:
;; @0047                               return
;; }
