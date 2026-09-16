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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 65 ""
;;     region3 = 237 ""
;;     region4 = 206 ""
;;     region5 = 196 ""
;;     region6 = 130 ""
;;     region7 = 6 ""
;;     region8 = 108 ""
;;     region9 = 5 ""
;;     region10 = 135 ""
;;     region11 = 187 ""
;;     region12 = 26 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v131 = stack_addr.i64 ss2
;;                                     store notrap aligned region12 v2, v131
;;                                     v132 = stack_addr.i64 ss1
;;                                     store notrap aligned region11 v3, v132
;;                                     v133 = stack_addr.i64 ss0
;;                                     store notrap aligned region10 v4, v133
;; @0025                               v17 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0025                               v18 = load.i32 notrap aligned region3 v17
;;                                     v151 = iconst.i32 7
;; @0025                               v21 = uadd_overflow_trap v18, v151, user18  ; v151 = 7
;;                                     v157 = iconst.i32 -8
;; @0025                               v23 = band v21, v157  ; v157 = -8
;;                                     v144 = iconst.i32 24
;; @0025                               v24 = uadd_overflow_trap v23, v144, user18  ; v144 = 24
;; @0025                               v26 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0025                               v27 = load.i64 notrap aligned region4 v26+40
;; @0025                               v25 = uextend.i64 v24
;; @0025                               v28 = icmp ule v25, v27
;; @0025                               brif v28, block2, block3
;;
;;                                 block2:
;;                                     v158 = iconst.i32 -1476394984
;; @0025                               v32 = load.i64 notrap aligned readonly can_move region5 v26+32
;;                                     v253 = band.i32 v21, v157  ; v157 = -8
;;                                     v254 = uextend.i64 v253
;; @0025                               v34 = iadd v32, v254
;; @0025                               store user2 region8 v158, v34  ; v158 = -1476394984
;; @0025                               v37 = load.i64 notrap aligned readonly can_move region6 v0+40
;; @0025                               v38 = load.i32 notrap aligned readonly can_move region7 v37
;; @0025                               store user2 region8 v38, v34+4
;; @0025                               store.i32 notrap aligned region3 v24, v17
;; @0025                               v5 = iconst.i32 3
;; @0025                               v39 = iconst.i64 8
;; @0025                               v40 = iadd v34, v39  ; v39 = 8
;; @0025                               store user2 region8 v5, v40  ; v5 = 3
;; @0025                               trapz v253, user16
;;                                     v255 = iconst.i32 24
;; @0025                               v61 = uadd_overflow_trap v253, v255, user2  ; v255 = 24
;;                                     v130 = load.i32 notrap aligned region12 v131
;; @0025                               v62 = uextend.i64 v61
;; @0025                               v65 = iadd v32, v62
;;                                     v135 = iconst.i64 12
;; @0025                               v68 = isub v65, v135  ; v135 = 12
;; @0025                               store user2 little region9 v130, v68
;;                                     v128 = load.i32 notrap aligned region11 v132
;; @0025                               v96 = isub v65, v39  ; v39 = 8
;; @0025                               store user2 little region9 v128, v96
;;                                     v126 = load.i32 notrap aligned region10 v133
;; @0025                               v8 = iconst.i64 4
;; @0025                               v124 = isub v65, v8  ; v8 = 4
;; @0025                               store user2 little region9 v126, v124
;; @0029                               jump block1
;;
;;                                 block3 cold:
;; @0025                               v29 = isub.i64 v25, v27
;; @0025                               v30 = call fn0(v0, v29), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]
;; @0025                               jump block2
;;
;;                                 block1:
;;                                     v256 = band.i32 v21, v157  ; v157 = -8
;; @0029                               return v256
;; }
