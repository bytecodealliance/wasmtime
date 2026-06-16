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
;;     region1 = 40 "VMContext+0x28"
;;     region2 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v15 = iconst.i32 -1476395008
;; @0025                               v16 = load.i64 notrap aligned readonly can_move region1 v0+40
;; @0025                               v17 = load.i32 notrap aligned readonly can_move v16
;;                                     v120 = iconst.i32 56
;; @0025                               v18 = iconst.i32 8
;; @0025                               v19 = call fn0(v0, v15, v17, v120, v18)  ; v15 = -1476395008, v120 = 56, v18 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v20 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v21 = load.i64 notrap aligned readonly can_move v20+32
;; @0025                               v22 = uextend.i64 v19
;; @0025                               v23 = iadd v21, v22
;;                                     v111 = iconst.i64 24
;; @0025                               v25 = iadd v23, v111  ; v111 = 24
;; @0025                               store user2 region2 v6, v25  ; v6 = 3
;; @0025                               trapz v19, user16
;; @0025                               v46 = uadd_overflow_trap v19, v120, user2  ; v120 = 56
;; @0025                               v47 = uextend.i64 v46
;; @0025                               v50 = iadd v21, v47
;; @0025                               v53 = isub v50, v111  ; v111 = 24
;; @0025                               store user2 little region2 v2, v53
;; @0025                               v61 = load.i32 user2 readonly region2 v25
;; @0025                               v54 = iconst.i32 1
;;                                     v151 = icmp ugt v61, v54  ; v54 = 1
;; @0025                               trapz v151, user17
;; @0025                               v64 = uextend.i64 v61
;;                                     v110 = iconst.i64 3
;;                                     v153 = ishl v64, v110  ; v110 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v67 = ushr v153, v11  ; v11 = 32
;; @0025                               trapnz v67, user2
;;                                     v160 = ishl v61, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 32
;; @0025                               v70 = uadd_overflow_trap v160, v7, user2  ; v7 = 32
;; @0025                               v74 = uadd_overflow_trap v19, v70, user2
;; @0025                               v75 = uextend.i64 v74
;; @0025                               v78 = iadd v21, v75
;;                                     v173 = iconst.i32 40
;; @0025                               v79 = isub v70, v173  ; v173 = 40
;; @0025                               v80 = uextend.i64 v79
;; @0025                               v81 = isub v78, v80
;; @0025                               store user2 little region2 v3, v81
;; @0025                               v89 = load.i32 user2 readonly region2 v25
;; @0025                               v82 = iconst.i32 2
;;                                     v179 = icmp ugt v89, v82  ; v82 = 2
;; @0025                               trapz v179, user17
;; @0025                               v92 = uextend.i64 v89
;;                                     v181 = ishl v92, v110  ; v110 = 3
;; @0025                               v95 = ushr v181, v11  ; v11 = 32
;; @0025                               trapnz v95, user2
;;                                     v188 = ishl v89, v6  ; v6 = 3
;; @0025                               v98 = uadd_overflow_trap v188, v7, user2  ; v7 = 32
;; @0025                               v102 = uadd_overflow_trap v19, v98, user2
;; @0025                               v103 = uextend.i64 v102
;; @0025                               v106 = iadd v21, v103
;;                                     v206 = iconst.i32 48
;; @0025                               v107 = isub v98, v206  ; v206 = 48
;; @0025                               v108 = uextend.i64 v107
;; @0025                               v109 = isub v106, v108
;; @0025                               store user2 little region2 v4, v109
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
