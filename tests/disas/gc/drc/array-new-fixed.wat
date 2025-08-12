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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1610612736:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v15 = iconst.i32 0
;;                                     v45 = iconst.i32 56
;; @0025                               v16 = iconst.i32 8
;; @0025                               v17 = call fn0(v0, v14, v15, v45, v16)  ; v14 = -1476395008, v15 = 0, v45 = 56, v16 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v30 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v18 = load.i64 notrap aligned readonly can_move v30+24
;; @0025                               v19 = uextend.i64 v17
;; @0025                               v20 = iadd v18, v19
;;                                     v35 = iconst.i64 24
;; @0025                               v21 = iadd v20, v35  ; v35 = 24
;; @0025                               store notrap aligned v6, v21  ; v6 = 3
;;                                     v32 = iconst.i64 32
;;                                     v52 = iadd v20, v32  ; v32 = 32
;; @0025                               store notrap aligned little v2, v52
;;                                     v54 = iconst.i64 40
;;                                     v60 = iadd v20, v54  ; v54 = 40
;; @0025                               store notrap aligned little v3, v60
;;                                     v62 = iconst.i64 48
;;                                     v68 = iadd v20, v62  ; v62 = 48
;; @0025                               store notrap aligned little v4, v68
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v17
;; }
