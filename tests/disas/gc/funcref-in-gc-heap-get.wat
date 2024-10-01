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
;; @0020                               trapz v2, null_reference
;; @0020                               v8 = uextend.i64 v2
;; @0020                               v9 = iconst.i64 16
;; @0020                               v10 = uadd_overflow_trap v8, v9, user65535  ; v9 = 16
;;                                     v19 = iconst.i64 24
;; @0020                               v12 = uadd_overflow_trap v8, v19, user65535  ; v19 = 24
;; @0020                               v7 = load.i64 notrap aligned readonly v0+48
;; @0020                               v13 = icmp ule v12, v7
;; @0020                               trapz v13, user65535
;; @0020                               v6 = load.i64 notrap aligned readonly v0+40
;; @0020                               v14 = iadd v6, v10
;; @0020                               v17 = load.i32 notrap aligned little v14
;; @0020                               v15 = iconst.i32 -1
;; @0020                               v18 = call fn0(v0, v17, v15)  ; v15 = -1
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v18
;; }
