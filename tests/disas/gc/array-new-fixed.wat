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
;;                                     v46 = iconst.i64 0
;; @0025                               trapnz v46, user18  ; v46 = 0
;; @0025                               v6 = iconst.i32 24
;; @0025                               v12 = uadd_overflow_trap v6, v6, user18  ; v6 = 24, v6 = 24
;; @0025                               v14 = iconst.i32 -1543503872
;; @0025                               v15 = iconst.i32 0
;; @0025                               v16 = iconst.i32 8
;; @0025                               v17 = call fn0(v0, v14, v15, v12, v16)  ; v14 = -1543503872, v15 = 0, v16 = 8
;; @0025                               v21 = uextend.i64 v17
;; @0025                               v22 = iconst.i64 16
;; @0025                               v23 = uadd_overflow_trap v21, v22, user1  ; v22 = 16
;; @0025                               v24 = uextend.i64 v12
;; @0025                               v25 = uadd_overflow_trap v21, v24, user1
;; @0025                               v20 = load.i64 notrap aligned readonly v0+48
;; @0025                               v26 = icmp ule v25, v20
;; @0025                               trapz v26, user1
;; @0025                               v7 = iconst.i32 3
;; @0025                               v19 = load.i64 notrap aligned readonly v0+40
;; @0025                               v27 = iadd v19, v23
;; @0025                               store notrap aligned v7, v27  ; v7 = 3
;;                                     v35 = iconst.i64 8
;; @0025                               v30 = iadd v27, v35  ; v35 = 8
;; @0025                               store notrap aligned little v2, v30
;;                                     v53 = iadd v27, v22  ; v22 = 16
;; @0025                               store notrap aligned little v3, v53
;;                                     v38 = iconst.i64 24
;;                                     v60 = iadd v27, v38  ; v38 = 24
;; @0025                               store notrap aligned little v4, v60
;; @0029                               jump block1
;;
;;                                 block1:
;; @0029                               return v17
;; }
