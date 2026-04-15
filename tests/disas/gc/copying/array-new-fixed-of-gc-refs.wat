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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;                                     v44 = stack_addr.i64 ss2
;;                                     store notrap v2, v44
;;                                     v43 = stack_addr.i64 ss1
;;                                     store notrap v3, v43
;;                                     v42 = stack_addr.i64 ss0
;;                                     store notrap v4, v42
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v16 = load.i64 notrap aligned readonly can_move v0+40
;; @0025                               v17 = load.i32 notrap aligned readonly can_move v16
;;                                     v57 = iconst.i32 32
;; @0025                               v18 = iconst.i32 16
;; @0025                               v19 = call fn0(v0, v14, v17, v57, v18), stack_map=[i32 @ ss2+0, i32 @ ss1+0, i32 @ ss0+0]  ; v14 = -1476395008, v57 = 32, v18 = 16
;; @0025                               v6 = iconst.i32 3
;; @0025                               v38 = load.i64 notrap aligned readonly can_move v0+8
;; @0025                               v20 = load.i64 notrap aligned readonly can_move v38+32
;; @0025                               v21 = uextend.i64 v19
;; @0025                               v22 = iadd v20, v21
;;                                     v37 = iconst.i64 16
;; @0025                               v23 = iadd v22, v37  ; v37 = 16
;; @0025                               store notrap aligned v6, v23  ; v6 = 3
;;                                     v33 = load.i32 notrap v44
;;                                     v59 = iconst.i64 20
;;                                     v65 = iadd v22, v59  ; v59 = 20
;; @0025                               store notrap aligned little v33, v65
;;                                     v32 = load.i32 notrap v43
;;                                     v68 = iconst.i64 24
;;                                     v74 = iadd v22, v68  ; v68 = 24
;; @0025                               store notrap aligned little v32, v74
;;                                     v31 = load.i32 notrap v42
;;                                     v92 = iconst.i64 28
;;                                     v98 = iadd v22, v92  ; v92 = 28
;; @0025                               store notrap aligned little v31, v98
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v19
;; }
