;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
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
;;     region11 = 1543503873 "Stack(ss1)"
;;     region12 = 1543503874 "Stack(ss2)"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v138 = stack_addr.i64 ss2
;;                                     store notrap aligned region12 v2, v138
;;                                     v139 = stack_addr.i64 ss1
;;                                     store notrap aligned region11 v3, v139
;;                                     v140 = stack_addr.i64 ss0
;;                                     store notrap aligned region10 v4, v140
;; @0025                               v14 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0025                               v15 = load.i32 notrap aligned region3 v14
;; @0025                               v16 = load.i32 notrap aligned region4 v14+4
;; @0025                               v22 = uextend.i64 v15
;; @0025                               v10 = iconst.i64 32
;; @0025                               v23 = iadd v22, v10  ; v10 = 32
;; @0025                               v24 = uextend.i64 v16
;; @0025                               v25 = icmp ule v23, v24
;; @0025                               brif v25, block2, block3
;;
;;                                 block2:
;;                                     v254 = iconst.i32 32
;;                                     v166 = iadd.i32 v15, v254  ; v254 = 32
;; @0025                               store notrap aligned region3 v166, v14
;;                                     v255 = iconst.i32 -1476394994
;;                                     v256 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v257 = load.i64 notrap aligned readonly can_move region7 v256+32
;; @0025                               v39 = iadd v257, v22
;; @0025                               store user2 region8 v255, v39  ; v255 = -1476394994
;;                                     v258 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v259 = load.i32 notrap aligned readonly can_move region6 v258
;; @0025                               store user2 region8 v259, v39+4
;; @0025                               store user2 region8 v254, v39+8  ; v254 = 32
;; @0025                               jump block4(v15, v39)
;;
;;                                 block3 cold:
;; @0025                               v26 = iconst.i32 -1476394994
;; @0025                               v27 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0025                               v28 = load.i32 notrap aligned readonly can_move region6 v27
;;                                     v152 = iconst.i32 32
;; @0025                               v29 = iconst.i32 16
;; @0025                               v30 = call fn0(v0, v26, v28, v152, v29), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v26 = -1476394994, v152 = 32, v29 = 16
;; @0025                               v31 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v32 = load.i64 notrap aligned readonly can_move region7 v31+32
;; @0025                               v33 = uextend.i64 v30
;; @0025                               v34 = iadd v32, v33
;; @0025                               jump block4(v30, v34)
;;
;;                                 block4(v44: i32, v45: i64):
;; @0025                               v5 = iconst.i32 3
;; @0025                               v46 = iconst.i64 16
;; @0025                               v47 = iadd v45, v46  ; v46 = 16
;; @0025                               store user2 region8 v5, v47  ; v5 = 3
;; @0025                               trapz v44, user16
;;                                     v260 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v261 = load.i64 notrap aligned readonly can_move region7 v260+32
;; @0025                               v49 = uextend.i64 v44
;; @0025                               v52 = iadd v261, v49
;; @0025                               v54 = iadd v52, v46  ; v46 = 16
;; @0025                               v55 = load.i32 user2 readonly region8 v54
;; @0025                               trapz v55, user17
;; @0025                               v58 = uextend.i64 v55
;;                                     v143 = iconst.i64 2
;;                                     v173 = ishl v58, v143  ; v143 = 2
;;                                     v262 = iconst.i64 32
;;                                     v263 = ushr v173, v262  ; v262 = 32
;; @0025                               trapnz v263, user2
;;                                     v180 = iconst.i32 2
;;                                     v181 = ishl v55, v180  ; v180 = 2
;; @0025                               v6 = iconst.i32 20
;; @0025                               v64 = uadd_overflow_trap v181, v6, user2  ; v6 = 20
;; @0025                               v68 = uadd_overflow_trap v44, v64, user2
;;                                     v137 = load.i32 notrap aligned region12 v138
;; @0025                               v69 = uextend.i64 v68
;; @0025                               v72 = iadd v261, v69
;; @0025                               v73 = isub v64, v6  ; v6 = 20
;; @0025                               v74 = uextend.i64 v73
;; @0025                               v75 = isub v72, v74
;; @0025                               store user2 little region8 v137, v75
;; @0025                               v83 = load.i32 user2 readonly region8 v54
;; @0025                               v76 = iconst.i32 1
;;                                     v197 = icmp ugt v83, v76  ; v76 = 1
;; @0025                               trapz v197, user17
;; @0025                               v86 = uextend.i64 v83
;;                                     v200 = ishl v86, v143  ; v143 = 2
;;                                     v264 = ushr v200, v262  ; v262 = 32
;; @0025                               trapnz v264, user2
;;                                     v205 = ishl v83, v180  ; v180 = 2
;; @0025                               v92 = uadd_overflow_trap v205, v6, user2  ; v6 = 20
;; @0025                               v96 = uadd_overflow_trap v44, v92, user2
;;                                     v135 = load.i32 notrap aligned region11 v139
;; @0025                               v97 = uextend.i64 v96
;; @0025                               v100 = iadd v261, v97
;;                                     v217 = iconst.i32 24
;; @0025                               v101 = isub v92, v217  ; v217 = 24
;; @0025                               v102 = uextend.i64 v101
;; @0025                               v103 = isub v100, v102
;; @0025                               store user2 little region8 v135, v103
;; @0025                               v111 = load.i32 user2 readonly region8 v54
;;                                     v223 = icmp ugt v111, v180  ; v180 = 2
;; @0025                               trapz v223, user17
;; @0025                               v114 = uextend.i64 v111
;;                                     v226 = ishl v114, v143  ; v143 = 2
;;                                     v265 = ushr v226, v262  ; v262 = 32
;; @0025                               trapnz v265, user2
;;                                     v231 = ishl v111, v180  ; v180 = 2
;; @0025                               v120 = uadd_overflow_trap v231, v6, user2  ; v6 = 20
;; @0025                               v124 = uadd_overflow_trap v44, v120, user2
;;                                     v133 = load.i32 notrap aligned region10 v140
;; @0025                               v125 = uextend.i64 v124
;; @0025                               v128 = iadd v261, v125
;;                                     v248 = iconst.i32 28
;; @0025                               v129 = isub v120, v248  ; v248 = 28
;; @0025                               v130 = uextend.i64 v129
;; @0025                               v131 = isub v128, v130
;; @0025                               store user2 little region8 v133, v131
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v44
;; }
