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
;;                                     v89 = iconst.i64 32
;; @0022                               v8 = ushr v92, v89  ; v89 = 32
;; @0022                               trapnz v8, user18
;; @0022                               v5 = iconst.i32 16
;;                                     v98 = iconst.i32 3
;;                                     v99 = ishl v3, v98  ; v98 = 3
;; @0022                               v10 = uadd_overflow_trap v5, v99, user18  ; v5 = 16
;; @0022                               v12 = iconst.i32 -67108864
;; @0022                               v13 = band v10, v12  ; v12 = -67108864
;; @0022                               trapnz v13, user18
;; @0022                               v15 = load.i64 notrap aligned readonly v0+32
;; @0022                               v16 = load.i32 user2 v15
;;                                     v102 = iconst.i32 7
;; @0022                               v19 = uadd_overflow_trap v16, v102, user18  ; v102 = 7
;;                                     v108 = iconst.i32 -8
;; @0022                               v21 = band v19, v108  ; v108 = -8
;; @0022                               v22 = uadd_overflow_trap v21, v10, user18
;; @0022                               v87 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v24 = load.i64 notrap aligned v87+40
;; @0022                               v23 = uextend.i64 v22
;; @0022                               v25 = icmp ule v23, v24
;; @0022                               brif v25, block2, block3
;;
;;                                 block2:
;; @0022                               v32 = iconst.i32 -1476395008
;;                                     v109 = bor.i32 v10, v32  ; v32 = -1476395008
;; @0022                               v29 = load.i64 notrap aligned readonly can_move v87+32
;;                                     v126 = band.i32 v19, v108  ; v108 = -8
;;                                     v127 = uextend.i64 v126
;; @0022                               v31 = iadd v29, v127
;; @0022                               store user2 v109, v31
;; @0022                               v35 = load.i64 notrap aligned readonly can_move v0+40
;; @0022                               v36 = load.i32 notrap aligned readonly can_move v35
;; @0022                               store user2 v36, v31+4
;; @0022                               store.i32 user2 v22, v15
;;                                     v90 = iconst.i64 8
;; @0022                               v37 = iadd v31, v90  ; v90 = 8
;; @0022                               store.i32 user2 v3, v37
;; @0022                               trapz v126, user16
;; @0022                               v61 = load.i64 notrap aligned v87+40
;;                                     v78 = iconst.i64 16
;; @0022                               v54 = iadd v31, v78  ; v78 = 16
;; @0022                               v63 = uadd_overflow_trap v54, v92, user2
;; @0022                               v62 = iadd v29, v61
;; @0022                               v64 = icmp ugt v63, v62
;; @0022                               trapnz v64, user2
;;                                     v111 = iconst.i64 0
;; @0022                               v66 = icmp.i64 eq v6, v111  ; v111 = 0
;; @0022                               v65 = iadd v54, v92
;; @0022                               brif v66, block5, block4(v54)
;;
;;                                 block4(v67: i64):
;; @0022                               store.i64 user2 little v2, v67
;;                                     v128 = iconst.i64 8
;;                                     v129 = iadd v67, v128  ; v128 = 8
;; @0022                               v69 = icmp eq v129, v65
;; @0022                               brif v69, block5, block4(v129)
;;
;;                                 block5:
;; @0025                               jump block1
;;
;;                                 block3 cold:
;; @0022                               v27 = isub.i64 v23, v24
;; @0022                               v28 = call fn0(v0, v27)
;; @0022                               jump block2
;;
;;                                 block1:
;;                                     v130 = band.i32 v19, v108  ; v108 = -8
;; @0025                               return v130
;; }
