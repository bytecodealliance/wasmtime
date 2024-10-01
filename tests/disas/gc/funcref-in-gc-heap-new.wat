;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut funcref))))

  (func (param funcref) (result (ref $ty))
    (struct.new $ty (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i64) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i32 uext, i32 uext) -> i32 system_v
;;     sig1 = (i64 vmctx, i64) -> i32 uext system_v
;;     fn0 = colocated u1:27 sig0
;;     fn1 = colocated u1:28 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i64):
;; @0020                               v6 = iconst.i32 -1476395008
;; @0020                               v7 = iconst.i32 0
;; @0020                               v4 = iconst.i32 24
;; @0020                               v8 = iconst.i32 8
;; @0020                               v9 = call fn0(v0, v6, v7, v4, v8)  ; v6 = -1476395008, v7 = 0, v4 = 24, v8 = 8
;;                                     v24 = stack_addr.i64 ss0
;;                                     store notrap v9, v24
;; @0020                               v13 = uextend.i64 v9
;; @0020                               v14 = iconst.i64 16
;; @0020                               v15 = uadd_overflow_trap v13, v14, user1  ; v14 = 16
;;                                     v27 = iconst.i64 24
;; @0020                               v17 = uadd_overflow_trap v13, v27, user1  ; v27 = 24
;; @0020                               v12 = load.i64 notrap aligned readonly v0+48
;; @0020                               v18 = icmp ule v17, v12
;; @0020                               trapz v18, user1
;; @0020                               v21 = call fn1(v0, v2), stack_map=[i32 @ ss0+0]
;; @0020                               v11 = load.i64 notrap aligned readonly v0+40
;; @0020                               v19 = iadd v11, v15
;; @0020                               store notrap aligned little v21, v19
;;                                     v22 = load.i32 notrap v24
;; @0023                               jump block1
;;
;;                                 block1:
;; @0023                               return v22
;; }
