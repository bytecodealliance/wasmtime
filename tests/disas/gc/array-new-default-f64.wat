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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 32 "VMContext+0x20"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:2 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               v5 = uextend.i64 v2
;;                                     v91 = iconst.i64 3
;;                                     v92 = ishl v5, v91  ; v91 = 3
;; @001f                               v8 = iconst.i64 32
;; @001f                               v9 = ushr v92, v8  ; v8 = 32
;; @001f                               trapnz v9, user18
;; @001f                               v4 = iconst.i32 24
;;                                     v98 = iconst.i32 3
;;                                     v99 = ishl v2, v98  ; v98 = 3
;; @001f                               v11 = uadd_overflow_trap v4, v99, user18  ; v4 = 24
;; @001f                               v12 = load.i64 notrap aligned readonly can_move region1 v0+32
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
;;                                     v107 = iconst.i32 15
;;                                     v108 = iadd.i32 v11, v107  ; v107 = 15
;;                                     v111 = iconst.i32 -16
;;                                     v112 = band v108, v111  ; v111 = -16
;;                                     v114 = iadd.i32 v13, v112
;; @001f                               store notrap aligned v114, v12
;;                                     v130 = iconst.i32 -1476395002
;;                                     v131 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v132 = load.i64 notrap aligned readonly can_move v131+32
;; @001f                               v37 = iadd v132, v20
;; @001f                               store notrap aligned v130, v37  ; v130 = -1476395002
;;                                     v133 = load.i64 notrap aligned readonly can_move region2 v0+40
;;                                     v134 = load.i32 notrap aligned readonly can_move v133
;; @001f                               store notrap aligned v134, v37+4
;;                                     v135 = band.i64 v18, v17  ; v17 = -16
;; @001f                               istore32 notrap aligned v135, v37+8
;; @001f                               jump block4(v13, v37)
;;
;;                                 block3 cold:
;; @001f                               v24 = iconst.i32 -1476395002
;; @001f                               v25 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @001f                               v26 = load.i32 notrap aligned readonly can_move v25
;; @001f                               v27 = iconst.i32 16
;; @001f                               v28 = call fn0(v0, v24, v26, v11, v27)  ; v24 = -1476395002, v27 = 16
;; @001f                               v29 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001f                               v30 = load.i64 notrap aligned readonly can_move v29+32
;; @001f                               v31 = uextend.i64 v28
;; @001f                               v32 = iadd v30, v31
;; @001f                               jump block4(v28, v32)
;;
;;                                 block4(v41: i32, v42: i64):
;;                                     v90 = stack_addr.i64 ss0
;;                                     store notrap v41, v90
;; @001f                               v43 = iconst.i64 16
;; @001f                               v44 = iadd v42, v43  ; v43 = 16
;; @001f                               store.i32 user2 region3 v2, v44
;; @001f                               trapz v41, user16
;;                                     v136 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v137 = load.i64 notrap aligned readonly can_move v136+32
;; @001f                               v47 = uextend.i64 v41
;; @001f                               v50 = iadd v137, v47
;; @001f                               v52 = iadd v50, v43  ; v43 = 16
;; @001f                               v53 = load.i32 user2 readonly region3 v52
;; @001f                               v54 = uextend.i64 v53
;; @001f                               v60 = icmp.i64 ugt v5, v54
;; @001f                               trapnz v60, user17
;; @001f                               v77 = load.i64 notrap aligned v136+40
;; @001f                               v65 = iconst.i64 24
;; @001f                               v66 = iadd v50, v65  ; v65 = 24
;; @001f                               v79 = uadd_overflow_trap v66, v92, user2
;; @001f                               v78 = iadd v137, v77
;; @001f                               v80 = icmp ugt v79, v78
;; @001f                               trapnz v80, user2
;; @001f                               v46 = iconst.i32 0
;; @001f                               call fn1(v0, v66, v46, v92), stack_map=[i32 @ ss0+0]  ; v46 = 0
;;                                     v83 = load.i32 notrap v90
;; @0022                               jump block1
;;
;;                                 block1:
;; @0022                               return v83
;; }
