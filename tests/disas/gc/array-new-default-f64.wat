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
;;                                     v102 = iconst.i64 3
;;                                     v103 = ishl v5, v102  ; v102 = 3
;;                                     v100 = iconst.i64 32
;; @001f                               v7 = ushr v103, v100  ; v100 = 32
;; @001f                               trapnz v7, user18
;; @001f                               v4 = iconst.i32 24
;;                                     v109 = iconst.i32 3
;;                                     v110 = ishl v2, v109  ; v109 = 3
;; @001f                               v9 = uadd_overflow_trap v4, v110, user18  ; v4 = 24
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
;;                                     v118 = iconst.i32 15
;;                                     v119 = iadd.i32 v9, v118  ; v118 = 15
;;                                     v122 = iconst.i32 -16
;;                                     v123 = band v119, v122  ; v122 = -16
;;                                     v125 = iadd.i32 v12, v123
;; @001f                               store notrap aligned region0 v125, v11
;;                                     v141 = iconst.i32 -1476395002
;;                                     v142 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v143 = load.i64 notrap aligned readonly can_move v142+32
;; @001f                               v36 = iadd v143, v19
;; @001f                               store notrap aligned v141, v36  ; v141 = -1476395002
;;                                     v144 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v145 = load.i32 notrap aligned readonly can_move v144
;; @001f                               store notrap aligned v145, v36+4
;;                                     v146 = band.i64 v17, v16  ; v16 = -16
;; @001f                               istore32 notrap aligned v146, v36+8
;; @001f                               jump block4(v12, v36)
;;
;;                                 block3 cold:
;; @001f                               v24 = iconst.i32 -1476395002
;; @001f                               v26 = load.i64 notrap aligned readonly can_move v0+40
;; @001f                               v27 = load.i32 notrap aligned readonly can_move v26
;; @001f                               v28 = iconst.i32 16
;; @001f                               v29 = call fn0(v0, v24, v27, v9, v28)  ; v24 = -1476395002, v28 = 16
;; @001f                               v96 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v30 = load.i64 notrap aligned readonly can_move v96+32
;; @001f                               v31 = uextend.i64 v29
;; @001f                               v32 = iadd v30, v31
;; @001f                               jump block4(v29, v32)
;;
;;                                 block4(v41: i32, v42: i64):
;;                                     v95 = stack_addr.i64 ss0
;;                                     store notrap v41, v95
;;                                     v94 = iconst.i64 16
;; @001f                               v43 = iadd v42, v94  ; v94 = 16
;; @001f                               store.i32 user2 v2, v43
;;                                     v77 = load.i32 notrap v95
;; @001f                               trapz v77, user16
;;                                     v147 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v148 = load.i64 notrap aligned readonly can_move v147+32
;; @001f                               v46 = uextend.i64 v77
;; @001f                               v48 = iadd v148, v46
;; @001f                               v50 = iadd v48, v94  ; v94 = 16
;; @001f                               v51 = load.i32 user2 readonly v50
;; @001f                               v52 = uextend.i64 v51
;; @001f                               v57 = icmp.i64 ugt v5, v52
;; @001f                               trapnz v57, user17
;; @001f                               v68 = load.i64 notrap aligned v147+40
;;                                     v85 = iconst.i64 24
;; @001f                               v61 = iadd v48, v85  ; v85 = 24
;; @001f                               v70 = uadd_overflow_trap v61, v103, user2
;; @001f                               v69 = iadd v148, v68
;; @001f                               v71 = icmp ugt v70, v69
;; @001f                               trapnz v71, user2
;; @001f                               v45 = iconst.i32 0
;; @001f                               call fn1(v0, v61, v45, v103), stack_map=[i32 @ ss0+0]  ; v45 = 0
;;                                     v74 = load.i32 notrap v95
;; @0022                               jump block1
;;
;;                                 block1:
;; @0022                               return v74
;; }
