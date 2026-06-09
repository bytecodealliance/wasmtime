;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param i64 i32) (result (ref $ty))
    (array.new $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i32) -> i32 tail {
;;     region0 = 32 "VMContext+0x20"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+40
;;     gv6 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i32):
;; @0022                               v6 = uextend.i64 v3
;;                                     v85 = iconst.i64 3
;;                                     v86 = ishl v6, v85  ; v85 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v86, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 16
;;                                     v92 = iconst.i32 3
;;                                     v93 = ishl v3, v92  ; v92 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v93, user18  ; v5 = 16
;; @0022                               v14 = iconst.i32 -67108864
;; @0022                               v15 = band v12, v14  ; v14 = -67108864
;; @0022                               trapnz v15, user18
;; @0022                               v16 = load.i64 notrap aligned readonly region0 v0+32
;; @0022                               v17 = load.i32 user2 region1 v16
;;                                     v96 = iconst.i32 7
;; @0022                               v20 = uadd_overflow_trap v17, v96, user18  ; v96 = 7
;;                                     v102 = iconst.i32 -8
;; @0022                               v22 = band v20, v102  ; v102 = -8
;; @0022                               v23 = uadd_overflow_trap v22, v12, user18
;; @0022                               v83 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v25 = load.i64 notrap aligned v83+40
;; @0022                               v24 = uextend.i64 v23
;; @0022                               v26 = icmp ule v24, v25
;; @0022                               brif v26, block2, block3
;;
;;                                 block2:
;; @0022                               v33 = iconst.i32 -1476395008
;;                                     v103 = bor.i32 v12, v33  ; v33 = -1476395008
;; @0022                               v30 = load.i64 notrap aligned readonly can_move v83+32
;;                                     v120 = band.i32 v20, v102  ; v102 = -8
;;                                     v121 = uextend.i64 v120
;; @0022                               v32 = iadd v30, v121
;; @0022                               store user2 region1 v103, v32
;; @0022                               v35 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v36 = load.i32 notrap aligned readonly can_move v35
;; @0022                               store user2 region1 v36, v32+4
;; @0022                               store.i32 user2 region1 v23, v16
;; @0022                               v7 = iconst.i64 8
;; @0022                               v38 = iadd v32, v7  ; v7 = 8
;; @0022                               store.i32 user2 region1 v3, v38
;; @0022                               trapz v120, user16
;; @0022                               v68 = load.i64 notrap aligned v83+40
;; @0022                               v57 = iconst.i64 16
;; @0022                               v58 = iadd v32, v57  ; v57 = 16
;; @0022                               v70 = uadd_overflow_trap v58, v86, user2
;; @0022                               v69 = iadd v30, v68
;; @0022                               v71 = icmp ugt v70, v69
;; @0022                               trapnz v71, user2
;;                                     v105 = iconst.i64 0
;; @0022                               v74 = icmp.i64 eq v6, v105  ; v105 = 0
;; @0022                               v72 = iadd v58, v86
;; @0022                               brif v74, block5, block4(v58)
;;
;;                                 block4(v75: i64):
;; @0022                               store.i64 user2 little region1 v2, v75
;;                                     v122 = iconst.i64 8
;;                                     v123 = iadd v75, v122  ; v122 = 8
;; @0022                               v78 = icmp eq v123, v72
;; @0022                               brif v78, block5, block4(v123)
;;
;;                                 block5:
;; @0025                               jump block1
;;
;;                                 block3 cold:
;; @0022                               v27 = isub.i64 v24, v25
;; @0022                               v28 = call fn0(v0, v27)
;; @0022                               jump block2
;;
;;                                 block1:
;;                                     v124 = band.i32 v20, v102  ; v102 = -8
;; @0025                               return v124
;; }
