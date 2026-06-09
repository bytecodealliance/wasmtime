;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param i64 i64 i64) (result (ref $ty))
    (array.new_fixed $ty 3 (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i64, i64, i64) -> i32 tail {
;;     region0 = 32 "VMContext+0x20"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+40
;;     gv6 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v18 = load.i64 notrap aligned readonly region0 v0+32
;; @0025                               v19 = load.i32 user2 region1 v18
;;                                     v150 = iconst.i32 7
;; @0025                               v22 = uadd_overflow_trap v19, v150, user18  ; v150 = 7
;;                                     v156 = iconst.i32 -8
;; @0025                               v24 = band v22, v156  ; v156 = -8
;;                                     v143 = iconst.i32 40
;; @0025                               v25 = uadd_overflow_trap v24, v143, user18  ; v143 = 40
;; @0025                               v131 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v27 = load.i64 notrap aligned v131+40
;; @0025                               v26 = uextend.i64 v25
;; @0025                               v28 = icmp ule v26, v27
;; @0025                               brif v28, block2, block3
;;
;;                                 block2:
;;                                     v157 = iconst.i32 -1476394968
;; @0025                               v32 = load.i64 notrap aligned readonly can_move v131+32
;;                                     v252 = band.i32 v22, v156  ; v156 = -8
;;                                     v253 = uextend.i64 v252
;; @0025                               v34 = iadd v32, v253
;; @0025                               store user2 region1 v157, v34  ; v157 = -1476394968
;; @0025                               v37 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v38 = load.i32 notrap aligned readonly can_move v37
;; @0025                               store user2 region1 v38, v34+4
;; @0025                               store.i32 user2 region1 v25, v18
;; @0025                               v6 = iconst.i32 3
;; @0025                               v9 = iconst.i64 8
;; @0025                               v40 = iadd v34, v9  ; v9 = 8
;; @0025                               store user2 region1 v6, v40  ; v6 = 3
;; @0025                               trapz v252, user16
;;                                     v254 = iconst.i32 40
;; @0025                               v60 = uadd_overflow_trap v252, v254, user2  ; v254 = 40
;; @0025                               v61 = uextend.i64 v60
;; @0025                               v63 = iadd v32, v61
;;                                     v134 = iconst.i64 24
;; @0025                               v66 = isub v63, v134  ; v134 = 24
;; @0025                               store.i64 user2 little region1 v2, v66
;; @0025                               v73 = load.i32 user2 readonly region1 v40
;; @0025                               v67 = iconst.i32 1
;;                                     v193 = icmp ugt v73, v67  ; v67 = 1
;; @0025                               trapz v193, user17
;; @0025                               v76 = uextend.i64 v73
;;                                     v133 = iconst.i64 3
;;                                     v195 = ishl v76, v133  ; v133 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v79 = ushr v195, v11  ; v11 = 32
;; @0025                               trapnz v79, user2
;;                                     v202 = ishl v73, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 16
;; @0025                               v82 = uadd_overflow_trap v202, v7, user2  ; v7 = 16
;; @0025                               v86 = uadd_overflow_trap v252, v82, user2
;; @0025                               v87 = uextend.i64 v86
;; @0025                               v89 = iadd v32, v87
;;                                     v142 = iconst.i32 24
;; @0025                               v90 = isub v82, v142  ; v142 = 24
;; @0025                               v91 = uextend.i64 v90
;; @0025                               v92 = isub v89, v91
;; @0025                               store.i64 user2 little region1 v3, v92
;; @0025                               v99 = load.i32 user2 readonly region1 v40
;; @0025                               v93 = iconst.i32 2
;;                                     v220 = icmp ugt v99, v93  ; v93 = 2
;; @0025                               trapz v220, user17
;; @0025                               v102 = uextend.i64 v99
;;                                     v222 = ishl v102, v133  ; v133 = 3
;; @0025                               v105 = ushr v222, v11  ; v11 = 32
;; @0025                               trapnz v105, user2
;;                                     v229 = ishl v99, v6  ; v6 = 3
;; @0025                               v108 = uadd_overflow_trap v229, v7, user2  ; v7 = 16
;; @0025                               v112 = uadd_overflow_trap v252, v108, user2
;; @0025                               v113 = uextend.i64 v112
;; @0025                               v115 = iadd v32, v113
;;                                     v246 = iconst.i32 32
;; @0025                               v116 = isub v108, v246  ; v246 = 32
;; @0025                               v117 = uextend.i64 v116
;; @0025                               v118 = isub v115, v117
;; @0025                               store.i64 user2 little region1 v4, v118
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v29 = isub.i64 v26, v27
;; @0025                               v30 = call fn0(v0, v29)
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v255 = band.i32 v22, v156  ; v156 = -8
;; @0029                               return v255
;; }
