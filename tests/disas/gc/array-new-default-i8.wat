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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 3489660928 "VMCopyingHeapData+0x0"
;;     region4 = 3489660932 "VMCopyingHeapData+0x4"
;;     region5 = 40 "VMContext+0x28"
;;     region6 = 268435488 "VMStoreContext+0x20"
;;     region7 = 2147483648 "GcHeap"
;;     region8 = 268435496 "VMStoreContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:24 sig0
;;     fn1 = colocated u805306368:2 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               v4 = uextend.i64 v2
;; @001f                               v7 = iconst.i64 32
;; @001f                               v8 = ushr v4, v7  ; v7 = 32
;; @001f                               trapnz v8, user18
;; @001f                               v3 = iconst.i32 20
;; @001f                               v10 = uadd_overflow_trap v3, v2, user18  ; v3 = 20
;; @001f                               v11 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @001f                               v12 = load.i32 notrap aligned region3 v11
;; @001f                               v13 = load.i32 notrap aligned region4 v11+4
;; @001f                               v19 = uextend.i64 v12
;; @001f                               v14 = uextend.i64 v10
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
;;                                     v96 = iconst.i32 15
;;                                     v97 = iadd.i32 v10, v96  ; v96 = 15
;;                                     v100 = iconst.i32 -16
;;                                     v101 = band v97, v100  ; v100 = -16
;;                                     v103 = iadd.i32 v12, v101
;; @001f                               store notrap aligned region3 v103, v11
;;                                     v117 = iconst.i32 -1476395002
;;                                     v118 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v119 = load.i64 notrap aligned readonly can_move region6 v118+32
;; @001f                               v36 = iadd v119, v19
;; @001f                               store user2 region7 v117, v36  ; v117 = -1476395002
;;                                     v120 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v121 = load.i32 notrap aligned readonly can_move v120
;; @001f                               store user2 region7 v121, v36+4
;;                                     v122 = band.i64 v17, v16  ; v16 = -16
;; @001f                               istore32 user2 region7 v122, v36+8
;; @001f                               jump block4(v12, v36)
;;
;;                                 block3 cold:
;; @001f                               v23 = iconst.i32 -1476395002
;; @001f                               v24 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @001f                               v25 = load.i32 notrap aligned readonly can_move v24
;; @001f                               v26 = iconst.i32 16
;; @001f                               v27 = call fn0(v0, v23, v25, v10, v26)  ; v23 = -1476395002, v26 = 16
;; @001f                               v28 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001f                               v29 = load.i64 notrap aligned readonly can_move region6 v28+32
;; @001f                               v30 = uextend.i64 v27
;; @001f                               v31 = iadd v29, v30
;; @001f                               jump block4(v27, v31)
;;
;;                                 block4(v40: i32, v41: i64):
;;                                     v88 = stack_addr.i64 ss0
;;                                     store notrap v40, v88
;; @001f                               v42 = iconst.i64 16
;; @001f                               v43 = iadd v41, v42  ; v42 = 16
;; @001f                               store.i32 user2 region7 v2, v43
;; @001f                               trapz v40, user16
;;                                     v123 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v124 = load.i64 notrap aligned readonly can_move region6 v123+32
;; @001f                               v46 = uextend.i64 v40
;; @001f                               v49 = iadd v124, v46
;; @001f                               v51 = iadd v49, v42  ; v42 = 16
;; @001f                               v52 = load.i32 user2 readonly region7 v51
;; @001f                               v53 = uextend.i64 v52
;; @001f                               v59 = icmp.i64 ugt v4, v53
;; @001f                               trapnz v59, user17
;; @001f                               v76 = load.i64 notrap aligned region8 v123+40
;; @001f                               v64 = iconst.i64 20
;; @001f                               v65 = iadd v49, v64  ; v64 = 20
;; @001f                               v78 = uadd_overflow_trap v65, v4, user2
;; @001f                               v77 = iadd v124, v76
;; @001f                               v79 = icmp ugt v78, v77
;; @001f                               trapnz v79, user2
;; @001f                               v44 = iconst.i32 0
;; @001f                               call fn1(v0, v65, v44, v4), stack_map=[i32 @ ss0+0]  ; v44 = 0
;; @0022                               jump block1
;;
;;                                 block1:
;;                                     v81 = load.i32 notrap v88
;; @0022                               return v81
;; }
