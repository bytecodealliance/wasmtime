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
;;                                     v88 = iconst.i64 3
;;                                     v89 = ishl v6, v88  ; v88 = 3
;; @0022                               v9 = iconst.i64 32
;; @0022                               v10 = ushr v89, v9  ; v9 = 32
;; @0022                               trapnz v10, user18
;; @0022                               v5 = iconst.i32 16
;;                                     v95 = iconst.i32 3
;;                                     v96 = ishl v3, v95  ; v95 = 3
;; @0022                               v12 = uadd_overflow_trap v5, v96, user18  ; v5 = 16
;; @0022                               v14 = iconst.i32 -67108864
;; @0022                               v15 = band v12, v14  ; v14 = -67108864
;; @0022                               trapnz v15, user18
;; @0022                               v16 = load.i64 notrap aligned readonly region0 v0+32
;; @0022                               v17 = load.i32 user2 region1 v16
;;                                     v99 = iconst.i32 7
;; @0022                               v20 = uadd_overflow_trap v17, v99, user18  ; v99 = 7
;;                                     v105 = iconst.i32 -8
;; @0022                               v22 = band v20, v105  ; v105 = -8
;; @0022                               v23 = uadd_overflow_trap v22, v12, user18
;; @0022                               v86 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v25 = load.i64 notrap aligned v86+40
;; @0022                               v24 = uextend.i64 v23
;; @0022                               v26 = icmp ule v24, v25
;; @0022                               brif v26, block2, block3
;;
;;                                 block2:
;; @0022                               v32 = iconst.i32 -1476395008
;;                                     v106 = bor.i32 v12, v32  ; v32 = -1476395008
;; @0022                               v29 = load.i64 notrap aligned readonly can_move v86+32
;;                                     v123 = band.i32 v20, v105  ; v105 = -8
;;                                     v124 = uextend.i64 v123
;; @0022                               v31 = iadd v29, v124
;; @0022                               store user2 region1 v106, v31
;; @0022                               v34 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v35 = load.i32 notrap aligned readonly can_move v34
;; @0022                               store user2 region1 v35, v31+4
;; @0022                               store.i32 user2 region1 v23, v16
;; @0022                               v7 = iconst.i64 8
;; @0022                               v37 = iadd v31, v7  ; v7 = 8
;; @0022                               store.i32 user2 region1 v3, v37
;; @0022                               trapz v123, user16
;; @0022                               v65 = load.i64 notrap aligned v86+40
;; @0022                               v55 = iconst.i64 16
;; @0022                               v56 = iadd v31, v55  ; v55 = 16
;; @0022                               v67 = uadd_overflow_trap v56, v89, user2
;; @0022                               v66 = iadd v29, v65
;; @0022                               v68 = icmp ugt v67, v66
;; @0022                               trapnz v68, user2
;;                                     v108 = iconst.i64 0
;; @0022                               v71 = icmp.i64 eq v6, v108  ; v108 = 0
;; @0022                               v69 = iadd v56, v89
;; @0022                               brif v71, block5, block4(v56)
;;
;;                                 block4(v72: i64):
;; @0022                               store.i64 user2 little region1 v2, v72
;;                                     v125 = iconst.i64 8
;;                                     v126 = iadd v72, v125  ; v125 = 8
;; @0022                               v75 = icmp eq v126, v69
;; @0022                               brif v75, block5, block4(v126)
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
;;                                     v127 = band.i32 v20, v105  ; v105 = -8
;; @0025                               return v127
;; }
