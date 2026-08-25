;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param i64 i64 i64) (result (ref $ty))
    (array.new_fixed $ty 3 (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i64, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 1677721600 "TypeIdsArray+0x0"
;;     region4 = 67108896 "VMStoreContext+0x20"
;;     region5 = 536870912 "GcHeap"
;;     region6 = 67108904 "VMStoreContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v15 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0025                               v16 = load.i32 notrap aligned readonly can_move region3 v15
;;                                     v119 = iconst.i32 56
;; @0025                               v17 = iconst.i32 8
;; @0025                               v18 = call fn0(v0, v14, v16, v119, v17)  ; v14 = -1476395008, v119 = 56, v17 = 8
;; @0025                               v5 = iconst.i32 3
;; @0025                               v19 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v20 = load.i64 notrap aligned readonly can_move region4 v19+32
;; @0025                               v21 = uextend.i64 v18
;; @0025                               v22 = iadd v20, v21
;;                                     v110 = iconst.i64 24
;; @0025                               v24 = iadd v22, v110  ; v110 = 24
;; @0025                               store user2 region5 v5, v24  ; v5 = 3
;; @0025                               trapz v18, user16
;; @0025                               v45 = uadd_overflow_trap v18, v119, user2  ; v119 = 56
;; @0025                               v46 = uextend.i64 v45
;; @0025                               v49 = iadd v20, v46
;; @0025                               v52 = isub v49, v110  ; v110 = 24
;; @0025                               store user2 little region5 v2, v52
;; @0025                               v60 = load.i32 user2 readonly region5 v24
;; @0025                               v53 = iconst.i32 1
;;                                     v150 = icmp ugt v60, v53  ; v53 = 1
;; @0025                               trapz v150, user17
;; @0025                               v63 = uextend.i64 v60
;;                                     v109 = iconst.i64 3
;;                                     v153 = ishl v63, v109  ; v109 = 3
;; @0025                               v10 = iconst.i64 32
;; @0025                               v66 = ushr v153, v10  ; v10 = 32
;; @0025                               trapnz v66, user2
;;                                     v158 = ishl v60, v5  ; v5 = 3
;; @0025                               v6 = iconst.i32 32
;; @0025                               v69 = uadd_overflow_trap v158, v6, user2  ; v6 = 32
;; @0025                               v73 = uadd_overflow_trap v18, v69, user2
;; @0025                               v74 = uextend.i64 v73
;; @0025                               v77 = iadd v20, v74
;;                                     v170 = iconst.i32 40
;; @0025                               v78 = isub v69, v170  ; v170 = 40
;; @0025                               v79 = uextend.i64 v78
;; @0025                               v80 = isub v77, v79
;; @0025                               store user2 little region5 v3, v80
;; @0025                               v88 = load.i32 user2 readonly region5 v24
;; @0025                               v81 = iconst.i32 2
;;                                     v176 = icmp ugt v88, v81  ; v81 = 2
;; @0025                               trapz v176, user17
;; @0025                               v91 = uextend.i64 v88
;;                                     v179 = ishl v91, v109  ; v109 = 3
;; @0025                               v94 = ushr v179, v10  ; v10 = 32
;; @0025                               trapnz v94, user2
;;                                     v184 = ishl v88, v5  ; v5 = 3
;; @0025                               v97 = uadd_overflow_trap v184, v6, user2  ; v6 = 32
;; @0025                               v101 = uadd_overflow_trap v18, v97, user2
;; @0025                               v102 = uextend.i64 v101
;; @0025                               v105 = iadd v20, v102
;;                                     v201 = iconst.i32 48
;; @0025                               v106 = isub v97, v201  ; v201 = 48
;; @0025                               v107 = uextend.i64 v106
;; @0025                               v108 = isub v105, v107
;; @0025                               store user2 little region5 v4, v108
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v18
;; }
