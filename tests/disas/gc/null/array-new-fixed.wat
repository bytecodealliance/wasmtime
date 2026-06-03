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
;;                                     v151 = iconst.i32 7
;; @0025                               v22 = uadd_overflow_trap v19, v151, user18  ; v151 = 7
;;                                     v157 = iconst.i32 -8
;; @0025                               v24 = band v22, v157  ; v157 = -8
;;                                     v144 = iconst.i32 40
;; @0025                               v25 = uadd_overflow_trap v24, v144, user18  ; v144 = 40
;; @0025                               v132 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v27 = load.i64 notrap aligned v132+40
;; @0025                               v26 = uextend.i64 v25
;; @0025                               v28 = icmp ule v26, v27
;; @0025                               brif v28, block2, block3
;;
;;                                 block2:
;;                                     v158 = iconst.i32 -1476394968
;; @0025                               v31 = load.i64 notrap aligned readonly can_move v132+32
;;                                     v253 = band.i32 v22, v157  ; v157 = -8
;;                                     v254 = uextend.i64 v253
;; @0025                               v33 = iadd v31, v254
;; @0025                               store user2 region1 v158, v33  ; v158 = -1476394968
;; @0025                               v36 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v37 = load.i32 notrap aligned readonly can_move v36
;; @0025                               store user2 region1 v37, v33+4
;; @0025                               store.i32 user2 region1 v25, v18
;; @0025                               v6 = iconst.i32 3
;; @0025                               v9 = iconst.i64 8
;; @0025                               v39 = iadd v33, v9  ; v9 = 8
;; @0025                               store user2 region1 v6, v39  ; v6 = 3
;; @0025                               trapz v253, user16
;;                                     v255 = iconst.i32 40
;; @0025                               v59 = uadd_overflow_trap v253, v255, user2  ; v255 = 40
;; @0025                               v60 = uextend.i64 v59
;; @0025                               v62 = iadd v31, v60
;;                                     v135 = iconst.i64 24
;; @0025                               v65 = isub v62, v135  ; v135 = 24
;; @0025                               store.i64 user2 little region1 v2, v65
;; @0025                               v72 = load.i32 user2 readonly region1 v39
;; @0025                               v66 = iconst.i32 1
;;                                     v194 = icmp ugt v72, v66  ; v66 = 1
;; @0025                               trapz v194, user17
;; @0025                               v75 = uextend.i64 v72
;;                                     v134 = iconst.i64 3
;;                                     v196 = ishl v75, v134  ; v134 = 3
;; @0025                               v11 = iconst.i64 32
;; @0025                               v78 = ushr v196, v11  ; v11 = 32
;; @0025                               trapnz v78, user2
;;                                     v203 = ishl v72, v6  ; v6 = 3
;; @0025                               v7 = iconst.i32 16
;; @0025                               v81 = uadd_overflow_trap v203, v7, user2  ; v7 = 16
;; @0025                               v85 = uadd_overflow_trap v253, v81, user2
;; @0025                               v86 = uextend.i64 v85
;; @0025                               v88 = iadd v31, v86
;;                                     v143 = iconst.i32 24
;; @0025                               v89 = isub v81, v143  ; v143 = 24
;; @0025                               v90 = uextend.i64 v89
;; @0025                               v91 = isub v88, v90
;; @0025                               store.i64 user2 little region1 v3, v91
;; @0025                               v98 = load.i32 user2 readonly region1 v39
;; @0025                               v92 = iconst.i32 2
;;                                     v221 = icmp ugt v98, v92  ; v92 = 2
;; @0025                               trapz v221, user17
;; @0025                               v101 = uextend.i64 v98
;;                                     v223 = ishl v101, v134  ; v134 = 3
;; @0025                               v104 = ushr v223, v11  ; v11 = 32
;; @0025                               trapnz v104, user2
;;                                     v230 = ishl v98, v6  ; v6 = 3
;; @0025                               v107 = uadd_overflow_trap v230, v7, user2  ; v7 = 16
;; @0025                               v111 = uadd_overflow_trap v253, v107, user2
;; @0025                               v112 = uextend.i64 v111
;; @0025                               v114 = iadd v31, v112
;;                                     v247 = iconst.i32 32
;; @0025                               v115 = isub v107, v247  ; v247 = 32
;; @0025                               v116 = uextend.i64 v115
;; @0025                               v117 = isub v114, v116
;; @0025                               store.i64 user2 little region1 v4, v117
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v29 = isub.i64 v26, v27
;; @0025                               v30 = call fn0(v0, v29)
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v256 = band.i32 v22, v157  ; v157 = -8
;; @0029                               return v256
;; }
