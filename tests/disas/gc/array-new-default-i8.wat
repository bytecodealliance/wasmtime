;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut i8)))

  (func $fill (param $len i32) (result (ref $a))
    (array.new_default $a (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 2 "vmctx"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:2 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               v5 = uextend.i64 v2
;;                                     v99 = iconst.i64 32
;; @001f                               v7 = ushr v5, v99  ; v99 = 32
;; @001f                               trapnz v7, user18
;; @001f                               v4 = iconst.i32 20
;; @001f                               v9 = uadd_overflow_trap v4, v2, user18  ; v4 = 20
;; @001f                               v11 = load.i64 notrap aligned readonly can_move v0+32
;; @001f                               v12 = load.i32 notrap aligned v11
;; @001f                               v13 = load.i32 notrap aligned v11+4
;; @001f                               v19 = uextend.i64 v12
;; @001f                               v14 = uextend.i64 v9
;; @001f                               v15 = iconst.i64 15
;; @001f                               v17 = iadd v14, v15  ; v15 = 15
;; @001f                               v16 = iconst.i64 -16
;; @001f                               v18 = band v17, v16  ; v16 = -16
;; @001f                               v20 = iadd v19, v18
;; @001f                               v21 = uextend.i64 v13
;; @001f                               v22 = icmp ule v20, v21
;; @001f                               brif v22, block2, block3
;;
;;                                 block2:
;;                                     v108 = iconst.i32 15
;;                                     v109 = iadd.i32 v9, v108  ; v108 = 15
;;                                     v112 = iconst.i32 -16
;;                                     v113 = band v109, v112  ; v112 = -16
;;                                     v115 = iadd.i32 v12, v113
;; @001f                               store notrap aligned region0 v115, v11
;;                                     v129 = iconst.i32 -1476395002
;;                                     v130 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v131 = load.i64 notrap aligned readonly can_move v130+32
;; @001f                               v36 = iadd v131, v19
;; @001f                               store notrap aligned v129, v36  ; v129 = -1476395002
;;                                     v132 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v133 = load.i32 notrap aligned readonly can_move v132
;; @001f                               store notrap aligned v133, v36+4
;;                                     v134 = band.i64 v17, v16  ; v16 = -16
;; @001f                               istore32 notrap aligned v134, v36+8
;; @001f                               jump block4(v12, v36)
;;
;;                                 block3 cold:
;; @001f                               v24 = iconst.i32 -1476395002
;; @001f                               v26 = load.i64 notrap aligned readonly can_move v0+40
;; @001f                               v27 = load.i32 notrap aligned readonly can_move v26
;; @001f                               v28 = iconst.i32 16
;; @001f                               v29 = call fn0(v0, v24, v27, v9, v28)  ; v24 = -1476395002, v28 = 16
;; @001f                               v95 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v30 = load.i64 notrap aligned readonly can_move v95+32
;; @001f                               v31 = uextend.i64 v29
;; @001f                               v32 = iadd v30, v31
;; @001f                               jump block4(v29, v32)
;;
;;                                 block4(v41: i32, v42: i64):
;;                                     v94 = stack_addr.i64 ss0
;;                                     store notrap v41, v94
;;                                     v93 = iconst.i64 16
;; @001f                               v43 = iadd v42, v93  ; v93 = 16
;; @001f                               store.i32 user2 v2, v43
;;                                     v76 = load.i32 notrap v94
;; @001f                               trapz v76, user16
;;                                     v135 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v136 = load.i64 notrap aligned readonly can_move v135+32
;; @001f                               v46 = uextend.i64 v76
;; @001f                               v48 = iadd v136, v46
;; @001f                               v50 = iadd v48, v93  ; v93 = 16
;; @001f                               v51 = load.i32 user2 readonly v50
;; @001f                               v52 = uextend.i64 v51
;; @001f                               v57 = icmp.i64 ugt v5, v52
;; @001f                               trapnz v57, user17
;; @001f                               v68 = load.i64 notrap aligned v135+40
;;                                     v84 = iconst.i64 20
;; @001f                               v61 = iadd v48, v84  ; v84 = 20
;; @001f                               v70 = uadd_overflow_trap v61, v5, user2
;; @001f                               v69 = iadd v136, v68
;; @001f                               v71 = icmp ugt v70, v69
;; @001f                               trapnz v71, user2
;; @001f                               v44 = iconst.i32 0
;; @001f                               call fn1(v0, v61, v44, v5), stack_map=[i32 @ ss0+0]  ; v44 = 0
;;                                     v73 = load.i32 notrap v94
;; @0022                               jump block1
;;
;;                                 block1:
;; @0022                               return v73
;; }
