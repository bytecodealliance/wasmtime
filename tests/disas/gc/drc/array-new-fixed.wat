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
;;     fn0 = colocated u1:28 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v15 = iconst.i32 0
;;                                     v44 = iconst.i32 48
;; @0025                               v16 = iconst.i32 8
;; @0025                               v17 = call fn0(v0, v14, v15, v44, v16)  ; v14 = -1476395008, v15 = 0, v44 = 48, v16 = 8
;; @0025                               v6 = iconst.i32 3
;; @0025                               v31 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v18 = load.i64 notrap aligned readonly can_move v31+24
;; @0025                               v19 = uextend.i64 v17
;; @0025                               v20 = iadd v18, v19
;;                                     v33 = iconst.i64 16
;; @0025                               v21 = iadd v20, v33  ; v33 = 16
;; @0025                               store notrap aligned v6, v21  ; v6 = 3
;;                                     v35 = iconst.i64 24
;;                                     v51 = iadd v20, v35  ; v35 = 24
;; @0025                               store notrap aligned little v2, v51
;;                                     v30 = iconst.i64 32
;;                                     v58 = iadd v20, v30  ; v30 = 32
;; @0025                               store notrap aligned little v3, v58
;;                                     v60 = iconst.i64 40
;;                                     v66 = iadd v20, v60  ; v60 = 40
;; @0025                               store notrap aligned little v4, v66
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v17
;; }
