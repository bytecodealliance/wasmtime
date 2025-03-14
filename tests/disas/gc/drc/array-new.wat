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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v37 = iconst.i64 3
;;                                     v38 = ishl v6, v37  ; v37 = 3
;;                                     v35 = iconst.i64 32
;; @0022                               v8 = ushr v38, v35  ; v35 = 32
;; @0022                               trapnz v8, user18
;; @0022                               v5 = iconst.i32 32
;;                                     v44 = iconst.i32 3
;;                                     v45 = ishl v3, v44  ; v44 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v45, user18  ; v5 = 32
;; @0022                               v13 = iconst.i32 -1476395008
;; @0022                               v11 = iconst.i32 0
;;                                     v42 = iconst.i32 8
;; @0022                               v17 = call fn0(v0, v13, v11, v10, v42)  ; v13 = -1476395008, v11 = 0, v42 = 8
;; @0022                               v20 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v18 = ireduce.i32 v17
;; @0022                               v21 = uextend.i64 v18
;; @0022                               v22 = iadd v20, v21
;;                                     v36 = iconst.i64 24
;; @0022                               v23 = iadd v22, v36  ; v36 = 24
;; @0022                               store notrap aligned v3, v23
;;                                     v60 = iadd v22, v35  ; v35 = 32
;; @0022                               v29 = uextend.i64 v10
;; @0022                               v30 = iadd v22, v29
;;                                     v34 = iconst.i64 8
;; @0022                               jump block2(v60)
;;
;;                                 block2(v31: i64):
;; @0022                               v32 = icmp eq v31, v30
;; @0022                               brif v32, block4, block3
;;
;;                                 block3:
;; @0022                               store.i64 notrap aligned little v2, v31
;;                                     v72 = iconst.i64 8
;;                                     v73 = iadd.i64 v31, v72  ; v72 = 8
;; @0022                               jump block2(v73)
;;
;;                                 block4:
;; @0025                               jump block1
;;
;;                                 block1:
;; @0025                               return v18
;; }
