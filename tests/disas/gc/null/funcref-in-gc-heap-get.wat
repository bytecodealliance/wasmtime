;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut funcref))))

  (func (param (ref $ty)) (result funcref)
    (struct.get $ty 0 (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i64 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move gv0+8
;;     gv2 = load.i64 notrap aligned readonly can_move gv1+24
;;     gv3 = load.i64 notrap aligned gv1+32
;;     sig0 = (i64 vmctx, i32, i32) -> i64 tail
;;     fn0 = colocated u1:30 sig0
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0020                               trapz v2, user16
;; @0020                               v13 = load.i64 notrap aligned readonly can_move v0+8
;; @0020                               v5 = load.i64 notrap aligned readonly can_move v13+24
;; @0020                               v4 = uextend.i64 v2
;; @0020                               v6 = iadd v5, v4
;; @0020                               v7 = iconst.i64 8
;; @0020                               v8 = iadd v6, v7  ; v7 = 8
;; @0020                               v11 = load.i32 notrap aligned little v8
;; @0020                               v9 = iconst.i32 -1
;; @0020                               v12 = call fn0(v0, v11, v9)  ; v9 = -1
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v12
;; }
