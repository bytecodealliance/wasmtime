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
;; @001f                               v8 = iconst.i64 32
;; @001f                               v9 = ushr v5, v8  ; v8 = 32
;; @001f                               trapnz v9, user18
;; @001f                               v4 = iconst.i32 20
;; @001f                               v11 = uadd_overflow_trap v4, v2, user18  ; v4 = 20
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
;;                                     v103 = iconst.i32 15
;;                                     v104 = iadd.i32 v11, v103  ; v103 = 15
;;                                     v107 = iconst.i32 -16
;;                                     v108 = band v104, v107  ; v107 = -16
;;                                     v110 = iadd.i32 v13, v108
;; @001f                               store notrap aligned region0 v110, v12
;;                                     v124 = iconst.i32 -1476395002
;;                                     v125 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v126 = load.i64 notrap aligned readonly can_move v125+32
;; @001f                               v35 = iadd v126, v20
;; @001f                               store notrap aligned v124, v35  ; v124 = -1476395002
;;                                     v127 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v128 = load.i32 notrap aligned readonly can_move v127
;; @001f                               store notrap aligned v128, v35+4
;;                                     v129 = band.i64 v18, v17  ; v17 = -16
;; @001f                               istore32 notrap aligned v129, v35+8
;; @001f                               jump block4(v13, v35)
;;
;;                                 block3 cold:
;; @001f                               v24 = iconst.i32 -1476395002
;; @001f                               v25 = load.i64 notrap aligned readonly can_move v0+40
;; @001f                               v26 = load.i32 notrap aligned readonly can_move v25
;; @001f                               v27 = iconst.i32 16
;; @001f                               v28 = call fn0(v0, v24, v26, v11, v27)  ; v24 = -1476395002, v27 = 16
;; @001f                               v92 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v29 = load.i64 notrap aligned readonly can_move v92+32
;; @001f                               v30 = uextend.i64 v28
;; @001f                               v31 = iadd v29, v30
;; @001f                               jump block4(v28, v31)
;;
;;                                 block4(v39: i32, v40: i64):
;;                                     v83 = stack_addr.i64 ss0
;;                                     store notrap v39, v83
;; @001f                               v41 = iconst.i64 16
;; @001f                               v42 = iadd v40, v41  ; v41 = 16
;; @001f                               store.i32 user2 region1 v2, v42
;; @001f                               trapz v39, user16
;;                                     v130 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v131 = load.i64 notrap aligned readonly can_move v130+32
;; @001f                               v45 = uextend.i64 v39
;; @001f                               v47 = iadd v131, v45
;; @001f                               v49 = iadd v47, v41  ; v41 = 16
;; @001f                               v50 = load.i32 user2 readonly region1 v49
;; @001f                               v51 = uextend.i64 v50
;; @001f                               v57 = icmp.i64 ugt v5, v51
;; @001f                               trapnz v57, user17
;; @001f                               v71 = load.i64 notrap aligned v130+40
;; @001f                               v61 = iconst.i64 20
;; @001f                               v62 = iadd v47, v61  ; v61 = 20
;; @001f                               v73 = uadd_overflow_trap v62, v5, user2
;; @001f                               v72 = iadd v131, v71
;; @001f                               v74 = icmp ugt v73, v72
;; @001f                               trapnz v74, user2
;; @001f                               v43 = iconst.i32 0
;; @001f                               call fn1(v0, v62, v43, v5), stack_map=[i32 @ ss0+0]  ; v43 = 0
;;                                     v76 = load.i32 notrap v83
;; @0022                               jump block1
;;
;;                                 block1:
;; @0022                               return v76
;; }
