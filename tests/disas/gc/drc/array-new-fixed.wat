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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 130 ""
;;     region3 = 6 ""
;;     region4 = 196 ""
;;     region5 = 108 ""
;;     region6 = 206 ""
;;     region7 = 5 ""
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
;; @0025                               store user2 little region7 v2, v52
;;                                     v178 = iconst.i64 16
;; @0025                               v80 = isub v49, v178  ; v178 = 16
;; @0025                               store user2 little region7 v3, v80
;; @0025                               v8 = iconst.i64 8
;; @0025                               v108 = isub v49, v8  ; v8 = 8
;; @0025                               store user2 little region7 v4, v108
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v18
;; }
