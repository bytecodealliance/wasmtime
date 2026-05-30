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
;;                                     v91 = iconst.i64 3
;;                                     v92 = ishl v6, v91  ; v91 = 3
;;                                     v90 = iconst.i64 32
;; @0022                               v9 = ushr v92, v90  ; v90 = 32
;; @0022                               trapnz v9, user18
;; @0022                               v5 = iconst.i32 16
;;                                     v98 = iconst.i32 3
;;                                     v99 = ishl v3, v98  ; v98 = 3
;; @0022                               v11 = uadd_overflow_trap v5, v99, user18  ; v5 = 16
;; @0022                               v13 = iconst.i32 -67108864
;; @0022                               v14 = band v11, v13  ; v13 = -67108864
;; @0022                               trapnz v14, user18
;; @0022                               v16 = load.i64 notrap aligned readonly region0 v0+32
;; @0022                               v17 = load.i32 user2 region1 v16
;;                                     v102 = iconst.i32 7
;; @0022                               v20 = uadd_overflow_trap v17, v102, user18  ; v102 = 7
;;                                     v108 = iconst.i32 -8
;; @0022                               v22 = band v20, v108  ; v108 = -8
;; @0022                               v23 = uadd_overflow_trap v22, v11, user18
;; @0022                               v88 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v25 = load.i64 notrap aligned v88+40
;; @0022                               v24 = uextend.i64 v23
;; @0022                               v26 = icmp ule v24, v25
;; @0022                               brif v26, block2, block3
;;
;;                                 block2:
;; @0022                               v33 = iconst.i32 -1476395008
;;                                     v109 = bor.i32 v11, v33  ; v33 = -1476395008
;; @0022                               v30 = load.i64 notrap aligned readonly can_move v88+32
;;                                     v126 = band.i32 v20, v108  ; v108 = -8
;;                                     v127 = uextend.i64 v126
;; @0022                               v32 = iadd v30, v127
;; @0022                               store user2 region1 v109, v32
;; @0022                               v36 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v37 = load.i32 notrap aligned readonly can_move v36
;; @0022                               store user2 region1 v37, v32+4
;; @0022                               store.i32 user2 region1 v23, v16
;; @0022                               v7 = iconst.i64 8
;; @0022                               v39 = iadd v32, v7  ; v7 = 8
;; @0022                               store.i32 user2 region1 v3, v39
;; @0022                               trapz v126, user16
;; @0022                               v67 = load.i64 notrap aligned v88+40
;; @0022                               v57 = iconst.i64 16
;; @0022                               v58 = iadd v32, v57  ; v57 = 16
;; @0022                               v69 = uadd_overflow_trap v58, v92, user2
;; @0022                               v68 = iadd v30, v67
;; @0022                               v70 = icmp ugt v69, v68
;; @0022                               trapnz v70, user2
;;                                     v111 = iconst.i64 0
;; @0022                               v73 = icmp.i64 eq v6, v111  ; v111 = 0
;; @0022                               v71 = iadd v58, v92
;; @0022                               brif v73, block5, block4(v58)
;;
;;                                 block4(v74: i64):
;; @0022                               store.i64 user2 little region1 v2, v74
;;                                     v128 = iconst.i64 8
;;                                     v129 = iadd v74, v128  ; v128 = 8
;; @0022                               v77 = icmp eq v129, v71
;; @0022                               brif v77, block5, block4(v129)
;;
;;                                 block5:
;; @0025                               jump block1
;;
;;                                 block3 cold:
;; @0022                               v28 = isub.i64 v24, v25
;; @0022                               v29 = call fn0(v0, v28)
;; @0022                               jump block2
;;
;;                                 block1:
;;                                     v130 = band.i32 v20, v108  ; v108 = -8
;; @0025                               return v130
;; }
