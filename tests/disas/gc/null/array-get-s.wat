;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (array (mut i8)))

  (func (param (ref $ty) i32) (result i32)
    (array.get_s $ty (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0022                               trapz v2, user16
;; @0022                               v34 = load.i64 notrap aligned readonly can_move v0+8
;; @0022                               v6 = load.i64 notrap aligned readonly can_move v34+24
;; @0022                               v5 = uextend.i64 v2
;; @0022                               v7 = iadd v6, v5
;; @0022                               v8 = iconst.i64 8
;; @0022                               v9 = iadd v7, v8  ; v8 = 8
;; @0022                               v10 = load.i32 notrap aligned readonly v9
;; @0022                               v11 = icmp ult v3, v10
;; @0022                               trapz v11, user17
;; @0022                               v13 = uextend.i64 v10
;;                                     v33 = iconst.i64 32
;; @0022                               v15 = ushr v13, v33  ; v33 = 32
;; @0022                               trapnz v15, user1
;; @0022                               v17 = iconst.i32 12
;; @0022                               v18 = uadd_overflow_trap v10, v17, user1  ; v17 = 12
;; @0022                               v22 = uadd_overflow_trap v2, v18, user1
;; @0022                               v23 = uextend.i64 v22
;; @0022                               v25 = iadd v6, v23
;; @0022                               v21 = iadd v3, v17  ; v17 = 12
;; @0022                               v26 = isub v18, v21
;; @0022                               v27 = uextend.i64 v26
;; @0022                               v28 = isub v25, v27
;; @0022                               v29 = load.i8 notrap aligned little v28
;; @0025                               jump block1
;;
;;                                 block1:
;; @0022                               v30 = sextend.i32 v29
;; @0025                               return v30
;; }
