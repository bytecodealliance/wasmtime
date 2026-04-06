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
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v16 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v17 = load.i32 notrap aligned readonly can_move v16
;;                                     v47 = iconst.i32 56
;; @0025                               v18 = iconst.i32 8
;; @0025                               v19 = call fn0(v0, v14, v17, v47, v18)  ; v14 = -1476395008, v47 = 56, v18 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v32 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v20 = load.i64 notrap aligned readonly can_move v32+32
;; @0025                               v21 = uextend.i64 v19
;; @0025                               v22 = iadd v20, v21
;;                                     v37 = iconst.i64 24
;; @0025                               v23 = iadd v22, v37  ; v37 = 24
;; @0025                               store notrap aligned v6, v23  ; v6 = 3
;;                                     v34 = iconst.i64 32
;;                                     v54 = iadd v22, v34  ; v34 = 32
;; @0025                               store notrap aligned little v2, v54
;;                                     v57 = iconst.i64 40
;;                                     v63 = iadd v22, v57  ; v57 = 40
;; @0025                               store notrap aligned little v3, v63
;;                                     v81 = iconst.i64 48
;;                                     v87 = iadd v22, v81  ; v81 = 48
;; @0025                               store notrap aligned little v4, v87
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
