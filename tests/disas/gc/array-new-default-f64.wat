;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut f64)))

  (func $fill (param $len i32) (result (ref $a))
    (array.new_default $a (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 32 "VMContext+0x20"
;;     region1 = 2147483648 "GcHeap"
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
;;                                     v97 = iconst.i64 3
;;                                     v98 = ishl v5, v97  ; v97 = 3
;; @001f                               v8 = iconst.i64 32
;; @001f                               v9 = ushr v98, v8  ; v8 = 32
;; @001f                               trapnz v9, user18
;; @001f                               v4 = iconst.i32 24
;;                                     v104 = iconst.i32 3
;;                                     v105 = ishl v2, v104  ; v104 = 3
;; @001f                               v11 = uadd_overflow_trap v4, v105, user18  ; v4 = 24
;; @001f                               v12 = load.i64 notrap aligned readonly can_move v0+32
;; @001f                               v13 = load.i32 notrap aligned v12
;; @001f                               v14 = load.i32 notrap aligned v12+4
;; @001f                               v20 = uextend.i64 v13
;; @001f                               v15 = uextend.i64 v11
;; @001f                               v16 = iconst.i64 15
;; @001f                               v18 = iadd v15, v16  ; v16 = 15
;; @001f                               v17 = iconst.i64 -16
;; @001f                               v19 = band v18, v17  ; v17 = -16
;; @001f                               v21 = iadd v20, v19
;; @001f                               v22 = uextend.i64 v14
;; @001f                               v23 = icmp ule v21, v22
;; @001f                               brif v23, block2, block3
;;
;;                                 block2:
;;                                     v113 = iconst.i32 15
;;                                     v114 = iadd.i32 v11, v113  ; v113 = 15
;;                                     v117 = iconst.i32 -16
;;                                     v118 = band v114, v117  ; v117 = -16
;;                                     v120 = iadd.i32 v13, v118
;; @001f                               store notrap aligned region0 v120, v12
;;                                     v136 = iconst.i32 -1476395002
;;                                     v137 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v138 = load.i64 notrap aligned readonly can_move v137+32
;; @001f                               v35 = iadd v138, v20
;; @001f                               store notrap aligned v136, v35  ; v136 = -1476395002
;;                                     v139 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v140 = load.i32 notrap aligned readonly can_move v139
;; @001f                               store notrap aligned v140, v35+4
;;                                     v141 = band.i64 v18, v17  ; v17 = -16
;; @001f                               istore32 notrap aligned v141, v35+8
;; @001f                               jump block4(v13, v35)
;;
;;                                 block3 cold:
;; @001f                               v24 = iconst.i32 -1476395002
;; @001f                               v25 = load.i64 notrap aligned readonly can_move v0+40
;; @001f                               v26 = load.i32 notrap aligned readonly can_move v25
;; @001f                               v27 = iconst.i32 16
;; @001f                               v28 = call fn0(v0, v24, v26, v11, v27)  ; v24 = -1476395002, v27 = 16
;; @001f                               v93 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v29 = load.i64 notrap aligned readonly can_move v93+32
;; @001f                               v30 = uextend.i64 v28
;; @001f                               v31 = iadd v29, v30
;; @001f                               jump block4(v28, v31)
;;
;;                                 block4(v39: i32, v40: i64):
;;                                     v84 = stack_addr.i64 ss0
;;                                     store notrap v39, v84
;; @001f                               v41 = iconst.i64 16
;; @001f                               v42 = iadd v40, v41  ; v41 = 16
;; @001f                               store.i32 user2 region1 v2, v42
;; @001f                               trapz v39, user16
;;                                     v142 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v143 = load.i64 notrap aligned readonly can_move v142+32
;; @001f                               v45 = uextend.i64 v39
;; @001f                               v47 = iadd v143, v45
;; @001f                               v49 = iadd v47, v41  ; v41 = 16
;; @001f                               v50 = load.i32 user2 readonly region1 v49
;; @001f                               v51 = uextend.i64 v50
;; @001f                               v57 = icmp.i64 ugt v5, v51
;; @001f                               trapnz v57, user17
;; @001f                               v71 = load.i64 notrap aligned v142+40
;; @001f                               v61 = iconst.i64 24
;; @001f                               v62 = iadd v47, v61  ; v61 = 24
;; @001f                               v73 = uadd_overflow_trap v62, v98, user2
;; @001f                               v72 = iadd v143, v71
;; @001f                               v74 = icmp ugt v73, v72
;; @001f                               trapnz v74, user2
;; @001f                               v44 = iconst.i32 0
;; @001f                               call fn1(v0, v62, v44, v98), stack_map=[i32 @ ss0+0]  ; v44 = 0
;;                                     v77 = load.i32 notrap v84
;; @0022                               jump block1
;;
;;                                 block1:
;; @0022                               return v77
;; }
