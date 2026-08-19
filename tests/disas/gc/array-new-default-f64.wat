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
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 872415232 "VMCopyingHeapData+0x0"
;;     region4 = 872415236 "VMCopyingHeapData+0x4"
;;     region5 = 40 "VMContext+0x28"
;;     region6 = 1677721600 "TypeIdsArray+0x0"
;;     region7 = 67108896 "VMStoreContext+0x20"
;;     region8 = 536870912 "GcHeap"
;;     region9 = 67108904 "VMStoreContext+0x28"
;;     region10 = 1543503872 "Stack(ss0)"
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
;;                                     v91 = iconst.i64 3
;;                                     v92 = ishl v4, v91  ; v91 = 3
;; @001f                               v7 = iconst.i64 32
;; @001f                               v8 = ushr v92, v7  ; v7 = 32
;; @001f                               trapnz v8, user18
;; @001f                               v3 = iconst.i32 24
;;                                     v98 = iconst.i32 3
;;                                     v99 = ishl v2, v98  ; v98 = 3
;; @001f                               v10 = uadd_overflow_trap v3, v99, user18  ; v3 = 24
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
;;                                     v107 = iconst.i32 15
;;                                     v108 = iadd.i32 v10, v107  ; v107 = 15
;;                                     v111 = iconst.i32 -16
;;                                     v112 = band v108, v111  ; v111 = -16
;;                                     v114 = iadd.i32 v12, v112
;; @001f                               store notrap aligned region3 v114, v11
;;                                     v130 = iconst.i32 -1476395002
;;                                     v131 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v132 = load.i64 notrap aligned readonly can_move region7 v131+32
;; @001f                               v36 = iadd v132, v19
;; @001f                               store user2 region8 v130, v36  ; v130 = -1476395002
;;                                     v133 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v134 = load.i32 notrap aligned readonly can_move region6 v133
;; @001f                               store user2 region8 v134, v36+4
;; @001f                               store user2 region8 v112, v36+8
;; @001f                               jump block4(v12, v36)
;;
;;                                 block3 cold:
;; @001f                               v23 = iconst.i32 -1476395002
;; @001f                               v24 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @001f                               v25 = load.i32 notrap aligned readonly can_move region6 v24
;; @001f                               v26 = iconst.i32 16
;; @001f                               v27 = call fn0(v0, v23, v25, v10, v26)  ; v23 = -1476395002, v26 = 16
;; @001f                               v28 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @001f                               v29 = load.i64 notrap aligned readonly can_move region7 v28+32
;; @001f                               v30 = uextend.i64 v27
;; @001f                               v31 = iadd v29, v30
;; @001f                               jump block4(v27, v31)
;;
;;                                 block4(v41: i32, v42: i64):
;;                                     v90 = stack_addr.i64 ss0
;;                                     store notrap aligned region10 v41, v90
;; @001f                               v43 = iconst.i64 16
;; @001f                               v44 = iadd v42, v43  ; v43 = 16
;; @001f                               store.i32 user2 region8 v2, v44
;; @001f                               trapz v41, user16
;;                                     v135 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v136 = load.i64 notrap aligned readonly can_move region7 v135+32
;; @001f                               v47 = uextend.i64 v41
;; @001f                               v50 = iadd v136, v47
;; @001f                               v52 = iadd v50, v43  ; v43 = 16
;; @001f                               v53 = load.i32 user2 readonly region8 v52
;; @001f                               v54 = uextend.i64 v53
;; @001f                               v60 = icmp.i64 ugt v4, v54
;; @001f                               trapnz v60, user17
;; @001f                               v77 = load.i64 notrap aligned region9 v135+40
;; @001f                               v65 = iconst.i64 24
;; @001f                               v66 = iadd v50, v65  ; v65 = 24
;; @001f                               v79 = uadd_overflow_trap v66, v92, user2
;; @001f                               v78 = iadd v136, v77
;; @001f                               v80 = icmp ugt v79, v78
;; @001f                               trapnz v80, user2
;; @001f                               v46 = iconst.i32 0
;; @001f                               call fn1(v0, v66, v46, v92), stack_map=[i32 @ ss0+0]  ; v46 = 0
;; @0022                               jump block1
;;
;;                                 block1:
;;                                     v83 = load.i32 notrap aligned region10 v90
;; @0022                               return v83
;; }
