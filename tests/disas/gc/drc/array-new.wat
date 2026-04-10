;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param i64 i32) (result (ref $ty))
    (array.new $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v37 = iconst.i64 3
;;                                     v38 = ishl v6, v37  ; v37 = 3
;;                                     v35 = iconst.i64 32
;; @0022                               v8 = ushr v38, v35  ; v35 = 32
;; @0022                               trapnz v8, user17
;; @0022                               v5 = iconst.i32 32
;;                                     v44 = iconst.i32 3
;;                                     v45 = ishl v3, v44  ; v44 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v45, user17  ; v5 = 32
;; @0022                               v12 = iconst.i32 -1476395008
;; @0022                               v14 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v15 = load.i32 notrap aligned readonly can_move v14
;;                                     v42 = iconst.i32 8
;; @0022                               v17 = call fn0(v0, v12, v15, v10, v42)  ; v12 = -1476395008, v42 = 8
;; @0022                               v33 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v18 = load.i64 notrap aligned readonly can_move v33+32
;; @0022                               v19 = uextend.i64 v17
;; @0022                               v20 = iadd v18, v19
;;                                     v32 = iconst.i64 24
;; @0022                               v21 = iadd v20, v32  ; v32 = 24
;; @0022                               store notrap aligned v3, v21
;;                                     v54 = iadd v20, v35  ; v35 = 32
;; @0022                               v27 = uextend.i64 v10
;; @0022                               v28 = iadd v20, v27
;;                                     v36 = iconst.i64 8
;; @0022                               jump block2(v54)
;;
;;                                 block2(v29: i64):
;; @0022                               v30 = icmp eq v29, v28
;; @0022                               brif v30, block4, block3
;;
;;                                 block3:
;; @0022                               store.i64 notrap aligned little v2, v29
;;                                     v59 = iconst.i64 8
;;                                     v60 = iadd.i64 v29, v59  ; v59 = 8
;; @0022                               jump block2(v60)
;;
;;                                 block4:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v17
;; }
