;;! target = "x86_64"
;;! flags = "-W function-references,gc"
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
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i32 uext, i32 uext) -> i32 system_v
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64, v3: i64, v4: i64):
;;                                     v42 = iconst.i64 0
;; @0025                               trapnz v42, user18  ; v42 = 0
;; @0025                               v6 = iconst.i32 24
;; @0025                               v12 = uadd_overflow_trap v6, v6, user18  ; v6 = 24, v6 = 24
;; @0025                               v14 = iconst.i32 -1476395008
;; @0025                               v15 = iconst.i32 0
;; @0025                               v16 = iconst.i32 8
;; @0025                               v17 = call fn0(v0, v14, v15, v12, v16)  ; v14 = -1476395008, v15 = 0, v16 = 8
;; @0025                               v7 = iconst.i32 3
;; @0025                               v19 = load.i64 notrap aligned readonly v0+40
;; @0025                               v20 = uextend.i64 v17
;; @0025                               v21 = iadd v19, v20
;;                                     v32 = iconst.i64 16
;; @0025                               v22 = iadd v21, v32  ; v32 = 16
;; @0025                               store notrap aligned v7, v22  ; v7 = 3
;;                                     v34 = iconst.i64 24
;;                                     v49 = iadd v21, v34  ; v34 = 24
;; @0025                               store notrap aligned little v2, v49
;;                                     v31 = iconst.i64 32
;;                                     v56 = iadd v21, v31  ; v31 = 32
;; @0025                               store notrap aligned little v3, v56
;;                                     v58 = iconst.i64 40
;;                                     v64 = iadd v21, v58  ; v58 = 40
;; @0025                               store notrap aligned little v4, v64
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v17
;; }
