;;! target = "x86_64"
;;! flags = "-W function-references,gc"
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
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i64) -> i32 uext system_v
;;     fn0 = colocated u1:28 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @0022                               trapz v2, user16
;; @0022                               v8 = uextend.i64 v2
;; @0022                               v9 = iconst.i64 16
;; @0022                               v10 = uadd_overflow_trap v8, v9, user1  ; v9 = 16
;;                                     v17 = iconst.i64 24
;; @0022                               v12 = uadd_overflow_trap v8, v17, user1  ; v17 = 24
;; @0022                               v7 = load.i64 notrap aligned readonly v0+48
;; @0022                               v13 = icmp ule v12, v7
;; @0022                               trapz v13, user1
;; @0022                               v16 = call fn0(v0, v3)
;; @0022                               v6 = load.i64 notrap aligned readonly v0+40
;; @0022                               v14 = iadd v6, v10
;; @0022                               store notrap aligned little v16, v14
;; @0026                               jump block1
;;
;;                                 block1:
;; @0026                               return
;; }
