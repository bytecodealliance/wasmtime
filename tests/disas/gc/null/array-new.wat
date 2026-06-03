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
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v92, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 16
;;                                     v98 = iconst.i32 3
;;                                     v99 = ishl v3, v98  ; v98 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v99, user18  ; v5 = 16
;; @0022                               v14 = iconst.i32 -67108864
;; @0022                               v15 = band v12, v14  ; v14 = -67108864
;; @0022                               trapnz v15, user18
;; @0022                               v17 = load.i64 notrap aligned readonly region0 v0+32
;; @0022                               v18 = load.i32 user2 region1 v17
;;                                     v102 = iconst.i32 7
;; @0022                               v21 = uadd_overflow_trap v18, v102, user18  ; v102 = 7
;;                                     v108 = iconst.i32 -8
;; @0022                               v23 = band v21, v108  ; v108 = -8
;; @0022                               v24 = uadd_overflow_trap v23, v12, user18
;; @0022                               v89 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v26 = load.i64 notrap aligned v89+40
;; @0022                               v25 = uextend.i64 v24
;; @0022                               v27 = icmp ule v25, v26
;; @0022                               brif v27, block2, block3
;;
;;                                 block2:
;; @0022                               v34 = iconst.i32 -1476395008
;;                                     v109 = bor.i32 v12, v34  ; v34 = -1476395008
;; @0022                               v31 = load.i64 notrap aligned readonly can_move v89+32
;;                                     v126 = band.i32 v21, v108  ; v108 = -8
;;                                     v127 = uextend.i64 v126
;; @0022                               v33 = iadd v31, v127
;; @0022                               store user2 region1 v109, v33
;; @0022                               v37 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v38 = load.i32 notrap aligned readonly can_move v37
;; @0022                               store user2 region1 v38, v33+4
;; @0022                               store.i32 user2 region1 v24, v17
;; @0022                               v7 = iconst.i64 8
;; @0022                               v40 = iadd v33, v7  ; v7 = 8
;; @0022                               store.i32 user2 region1 v3, v40
;; @0022                               trapz v126, user16
;; @0022                               v68 = load.i64 notrap aligned v89+40
;; @0022                               v58 = iconst.i64 16
;; @0022                               v59 = iadd v33, v58  ; v58 = 16
;; @0022                               v70 = uadd_overflow_trap v59, v92, user2
;; @0022                               v69 = iadd v31, v68
;; @0022                               v71 = icmp ugt v70, v69
;; @0022                               trapnz v71, user2
;;                                     v111 = iconst.i64 0
;; @0022                               v74 = icmp.i64 eq v6, v111  ; v111 = 0
;; @0022                               v72 = iadd v59, v92
;; @0022                               brif v74, block5, block4(v59)
;;
;;                                 block4(v75: i64):
;; @0022                               store.i64 user2 little region1 v2, v75
;;                                     v128 = iconst.i64 8
;;                                     v129 = iadd v75, v128  ; v128 = 8
;; @0022                               v78 = icmp eq v129, v72
;; @0022                               brif v78, block5, block4(v129)
;;
;;                                 block5:
;; @0025                               jump block1
;;
;;                                 block3 cold:
;; @0022                               v29 = isub.i64 v25, v26
;; @0022                               v30 = call fn0(v0, v29)
;; @0022                               jump block2
;;
;;                                 block1:
;;                                     v130 = band.i32 v21, v108  ; v108 = -8
;; @0025                               return v130
;; }
