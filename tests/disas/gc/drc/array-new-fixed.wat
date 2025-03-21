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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;;                                     v44 = iconst.i64 0
;; @0025                               trapnz v44, user18  ; v44 = 0
;; @0025                               v7 = iconst.i32 32
;;                                     v45 = iconst.i32 24
;; @0025                               v12 = uadd_overflow_trap v7, v45, user18  ; v7 = 32, v45 = 24
;; @0025                               v15 = iconst.i32 -1476395008
;; @0025                               v13 = iconst.i32 0
;; @0025                               v18 = iconst.i32 8
;; @0025                               v19 = call fn0(v0, v15, v13, v12, v18)  ; v15 = -1476395008, v13 = 0, v18 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v21 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v22 = uextend.i64 v19
;; @0025                               v23 = iadd v21, v22
;;                                     v36 = iconst.i64 24
;; @0025                               v24 = iadd v23, v36  ; v36 = 24
;; @0025                               store notrap aligned v6, v24  ; v6 = 3
;;                                     v33 = iconst.i64 32
;;                                     v58 = iadd v23, v33  ; v33 = 32
;; @0025                               store notrap aligned little v2, v58
;;                                     v60 = iconst.i64 40
;;                                     v66 = iadd v23, v60  ; v60 = 40
;; @0025                               store notrap aligned little v3, v66
;;                                     v68 = iconst.i64 48
;;                                     v74 = iadd v23, v68  ; v68 = 48
;; @0025                               store notrap aligned little v4, v74
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
