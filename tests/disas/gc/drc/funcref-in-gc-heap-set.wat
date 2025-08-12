;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut funcref))))

  (func (param (ref $ty) funcref)
    (struct.set $ty 0 (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u1610612736:28 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @0022                               trapz v2, user16
;; @0022                               v10 = call fn0(v0, v3)
;; @0022                               v11 = ireduce.i32 v10
;; @0022                               v12 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v5 = load.i64 notrap aligned readonly can_move v12+24
;; @0022                               v4 = uextend.i64 v2
;; @0022                               v6 = iadd v5, v4
;; @0022                               v7 = iconst.i64 24
;; @0022                               v8 = iadd v6, v7  ; v7 = 24
;; @0022                               store notrap aligned little v11, v8
;; @0026                               jump block1
;;
;;                                 block1:
;; @0026                               return
;; }
