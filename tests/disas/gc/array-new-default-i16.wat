;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut i16)))

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
;;                                     v103 = iconst.i64 1
;;                                     v104 = ishl v5, v103  ; v103 = 1
;;                                     v101 = iconst.i64 32
;; @001f                               v8 = ushr v104, v101  ; v101 = 32
;; @001f                               trapnz v8, user18
;; @001f                               v4 = iconst.i32 20
;;                                     v108 = iadd v2, v2
;; @001f                               v10 = uadd_overflow_trap v4, v108, user18  ; v4 = 20
;; @001f                               v12 = load.i64 notrap aligned readonly can_move v0+32
;; @001f                               v13 = load.i32 notrap aligned v12
;; @001f                               v14 = load.i32 notrap aligned v12+4
;; @001f                               v20 = uextend.i64 v13
;; @001f                               v15 = uextend.i64 v10
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
;;                                     v123 = iconst.i32 15
;;                                     v124 = iadd.i32 v10, v123  ; v123 = 15
;;                                     v127 = iconst.i32 -16
;;                                     v128 = band v124, v127  ; v127 = -16
;;                                     v130 = iadd.i32 v13, v128
;; @001f                               store notrap aligned region0 v130, v12
;;                                     v151 = iconst.i32 -1476395002
;;                                     v152 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v153 = load.i64 notrap aligned readonly can_move v152+32
;; @001f                               v37 = iadd v153, v20
;; @001f                               store notrap aligned v151, v37  ; v151 = -1476395002
;;                                     v154 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v155 = load.i32 notrap aligned readonly can_move v154
;; @001f                               store notrap aligned v155, v37+4
;;                                     v156 = band.i64 v18, v17  ; v17 = -16
;; @001f                               istore32 notrap aligned v156, v37+8
;; @001f                               jump block4(v13, v37)
;;
;;                                 block3 cold:
;; @001f                               v25 = iconst.i32 -1476395002
;; @001f                               v27 = load.i64 notrap aligned readonly can_move v0+40
;; @001f                               v28 = load.i32 notrap aligned readonly can_move v27
;; @001f                               v29 = iconst.i32 16
;; @001f                               v30 = call fn0(v0, v25, v28, v10, v29)  ; v25 = -1476395002, v29 = 16
;; @001f                               v97 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v31 = load.i64 notrap aligned readonly can_move v97+32
;; @001f                               v32 = uextend.i64 v30
;; @001f                               v33 = iadd v31, v32
;; @001f                               jump block4(v30, v33)
;;
;;                                 block4(v42: i32, v43: i64):
;;                                     v96 = stack_addr.i64 ss0
;;                                     store notrap v42, v96
;; @001f                               v44 = iconst.i64 16
;; @001f                               v45 = iadd v43, v44  ; v44 = 16
;; @001f                               store.i32 user2 region1 v2, v45
;; @001f                               trapz v42, user16
;;                                     v157 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v158 = load.i64 notrap aligned readonly can_move v157+32
;; @001f                               v48 = uextend.i64 v42
;; @001f                               v50 = iadd v158, v48
;; @001f                               v52 = iadd v50, v44  ; v44 = 16
;; @001f                               v53 = load.i32 user2 readonly region1 v52
;; @001f                               v54 = uextend.i64 v53
;; @001f                               v60 = icmp.i64 ugt v5, v54
;; @001f                               trapnz v60, user17
;; @001f                               v74 = load.i64 notrap aligned v157+40
;; @001f                               v64 = iconst.i64 20
;; @001f                               v65 = iadd v50, v64  ; v64 = 20
;; @001f                               v76 = uadd_overflow_trap v65, v104, user2
;; @001f                               v75 = iadd v158, v74
;; @001f                               v77 = icmp ugt v76, v75
;; @001f                               trapnz v77, user2
;; @001f                               v46 = iconst.i32 0
;; @001f                               call fn1(v0, v65, v46, v104), stack_map=[i32 @ ss0+0]  ; v46 = 0
;;                                     v80 = load.i32 notrap v96
;; @0022                               jump block1
;;
;;                                 block1:
;; @0022                               return v80
;; }
