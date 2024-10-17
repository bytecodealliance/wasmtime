;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut funcref))))

  (func (param (ref $ty)) (result funcref)
    (struct.get $ty 0 (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext) -> i64 system_v
;;     fn0 = colocated u1:29 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0020                               trapz v2, user16
;; @0020                               v9 = uextend.i64 v2
;; @0020                               v10 = iconst.i64 16
;; @0020                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 16
;;                                     v20 = iconst.i64 24
;; @0020                               v13 = uadd_overflow_trap v9, v20, user1  ; v20 = 24
;; @0020                               v8 = load.i64 notrap aligned readonly v0+48
;; @0020                               v14 = icmp ule v13, v8
;; @0020                               trapz v14, user1
;; @0020                               v6 = load.i64 notrap aligned readonly v0+40
;; @0020                               v15 = iadd v6, v11
;; @0020                               v18 = load.i32 notrap aligned little v15
;; @0020                               v16 = iconst.i32 -1
;; @0020                               v19 = call fn0(v0, v18, v16)  ; v16 = -1
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v19
;; }
