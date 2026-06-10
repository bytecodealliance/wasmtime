;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (array (mut anyref)))

  (func (param anyref anyref anyref) (result (ref $ty))
    (array.new_fixed $ty 3 (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     ss1 = explicit_slot 4, align = 4
;;     ss2 = explicit_slot 4, align = 4
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 32 "VMContext+0x20"
;;     region2 = 2147483648 "GcHeap"
;;     region3 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v126 = stack_addr.i64 ss2
;;                                     store notrap v2, v126
;;                                     v127 = stack_addr.i64 ss1
;;                                     store notrap v3, v127
;;                                     v128 = stack_addr.i64 ss0
;;                                     store notrap v4, v128
;; @0025                               v18 = load.i64 notrap aligned readonly region1 v0+32
;; @0025                               v19 = load.i32 user2 region2 v18
;;                                     v158 = iconst.i32 7
;; @0025                               v22 = uadd_overflow_trap v19, v158, user18  ; v158 = 7
;;                                     v164 = iconst.i32 -8
;; @0025                               v24 = band v22, v164  ; v164 = -8
;;                                     v151 = iconst.i32 24
;; @0025                               v25 = uadd_overflow_trap v24, v151, user18  ; v151 = 24
;; @0025                               v27 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v28 = load.i64 notrap aligned v27+40
;; @0025                               v26 = uextend.i64 v25
;; @0025                               v29 = icmp ule v26, v28
;; @0025                               brif v29, block2, block3
;;
;;                                 block2:
;;                                     v165 = iconst.i32 -1476394984
;; @0025                               v33 = load.i64 notrap aligned readonly can_move v27+32
;;                                     v263 = band.i32 v22, v164  ; v164 = -8
;;                                     v264 = uextend.i64 v263
;; @0025                               v35 = iadd v33, v264
;; @0025                               store user2 region2 v165, v35  ; v165 = -1476394984
;; @0025                               v38 = load.i64 notrap aligned readonly can_move region3 v0+40
;; @0025                               v39 = load.i32 notrap aligned readonly can_move v38
;; @0025                               store user2 region2 v39, v35+4
;; @0025                               store.i32 user2 region2 v25, v18
;; @0025                               v6 = iconst.i32 3
;; @0025                               v40 = iconst.i64 8
;; @0025                               v41 = iadd v35, v40  ; v40 = 8
;; @0025                               store user2 region2 v6, v41  ; v6 = 3
;; @0025                               trapz v263, user16
;;                                     v265 = iconst.i32 24
;; @0025                               v61 = uadd_overflow_trap v263, v265, user2  ; v265 = 24
;;                                     v125 = load.i32 notrap v126
;; @0025                               v62 = uextend.i64 v61
;; @0025                               v64 = iadd v33, v62
;;                                     v142 = iconst.i64 12
;; @0025                               v67 = isub v64, v142  ; v142 = 12
;; @0025                               store user2 little region2 v125, v67
;; @0025                               v74 = load.i32 user2 readonly region2 v41
;; @0025                               v68 = iconst.i32 1
;;                                     v203 = icmp ugt v74, v68  ; v68 = 1
;; @0025                               trapz v203, user17
;; @0025                               v77 = uextend.i64 v74
;;                                     v143 = iconst.i64 2
;;                                     v205 = ishl v77, v143  ; v143 = 2
;; @0025                               v11 = iconst.i64 32
;; @0025                               v80 = ushr v205, v11  ; v11 = 32
;; @0025                               trapnz v80, user2
;;                                     v182 = iconst.i32 2
;;                                     v212 = ishl v74, v182  ; v182 = 2
;; @0025                               v7 = iconst.i32 12
;; @0025                               v83 = uadd_overflow_trap v212, v7, user2  ; v7 = 12
;; @0025                               v87 = uadd_overflow_trap v263, v83, user2
;;                                     v123 = load.i32 notrap v127
;; @0025                               v88 = uextend.i64 v87
;; @0025                               v90 = iadd v33, v88
;;                                     v225 = iconst.i32 16
;; @0025                               v91 = isub v83, v225  ; v225 = 16
;; @0025                               v92 = uextend.i64 v91
;; @0025                               v93 = isub v90, v92
;; @0025                               store user2 little region2 v123, v93
;; @0025                               v100 = load.i32 user2 readonly region2 v41
;;                                     v231 = icmp ugt v100, v182  ; v182 = 2
;; @0025                               trapz v231, user17
;; @0025                               v103 = uextend.i64 v100
;;                                     v233 = ishl v103, v143  ; v143 = 2
;; @0025                               v106 = ushr v233, v11  ; v11 = 32
;; @0025                               trapnz v106, user2
;;                                     v240 = ishl v100, v182  ; v182 = 2
;; @0025                               v109 = uadd_overflow_trap v240, v7, user2  ; v7 = 12
;; @0025                               v113 = uadd_overflow_trap v263, v109, user2
;;                                     v121 = load.i32 notrap v128
;; @0025                               v114 = uextend.i64 v113
;; @0025                               v116 = iadd v33, v114
;;                                     v257 = iconst.i32 20
;; @0025                               v117 = isub v109, v257  ; v257 = 20
;; @0025                               v118 = uextend.i64 v117
;; @0025                               v119 = isub v116, v118
;; @0025                               store user2 little region2 v121, v119
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v30 = isub.i64 v26, v28
;; @0025                               v31 = call fn0(v0, v30), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v266 = band.i32 v22, v164  ; v164 = -8
;; @0029                               return v266
;; }
