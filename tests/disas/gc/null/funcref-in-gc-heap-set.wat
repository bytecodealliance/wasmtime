;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
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
;;     sig0 = (i64 vmctx, i64) -> i64 tail
;;     fn0 = colocated u1:28 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i64):
;; @0022                               trapz v2, user16
;; @0022                               v9 = uextend.i64 v2
;; @0022                               v10 = iconst.i64 8
;; @0022                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 8
;;                                     v19 = iconst.i64 16
;; @0022                               v13 = uadd_overflow_trap v9, v19, user1  ; v19 = 16
;; @0022                               v8 = load.i64 notrap aligned readonly v0+48
;; @0022                               v14 = icmp ule v13, v8
;; @0022                               trapz v14, user1
;; @0022                               v17 = call fn0(v0, v3)
;; @0022                               v18 = ireduce.i32 v17
;; @0022                               v6 = load.i64 notrap aligned readonly v0+40
;; @0022                               v15 = iadd v6, v11
;; @0022                               store notrap aligned little v18, v15
;; @0026                               jump block1
;;
;;                                 block1:
;; @0026                               return
;; }
