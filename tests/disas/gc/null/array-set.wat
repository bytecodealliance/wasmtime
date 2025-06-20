;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32 i64)
    (array.set $ty (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i64) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64):
;; @0024                               trapz v2, user16
;; @0024                               v32 = load.i64 notrap aligned readonly can_move v0+8
;; @0024                               v6 = load.i64 notrap aligned readonly can_move v32+24
;; @0024                               v5 = uextend.i64 v2
;; @0024                               v7 = iadd v6, v5
;; @0024                               v8 = iconst.i64 8
;; @0024                               v9 = iadd v7, v8  ; v8 = 8
;; @0024                               v10 = load.i32 notrap aligned readonly v9
;; @0024                               v11 = icmp ult v3, v10
;; @0024                               trapz v11, user17
;; @0024                               v13 = uextend.i64 v10
;;                                     v34 = iconst.i64 3
;;                                     v35 = ishl v13, v34  ; v34 = 3
;;                                     v31 = iconst.i64 32
;; @0024                               v15 = ushr v35, v31  ; v31 = 32
;; @0024                               trapnz v15, user1
;;                                     v44 = iconst.i32 3
;;                                     v45 = ishl v10, v44  ; v44 = 3
;; @0024                               v17 = iconst.i32 16
;; @0024                               v18 = uadd_overflow_trap v45, v17, user1  ; v17 = 16
;; @0024                               v22 = uadd_overflow_trap v2, v18, user1
;; @0024                               v23 = uextend.i64 v22
;; @0024                               v25 = iadd v6, v23
;;                                     v51 = ishl v3, v44  ; v44 = 3
;; @0024                               v21 = iadd v51, v17  ; v17 = 16
;; @0024                               v26 = isub v18, v21
;; @0024                               v27 = uextend.i64 v26
;; @0024                               v28 = isub v25, v27
;; @0024                               store notrap aligned little v4, v28
;; @0027                               jump block1
;;
;;                                 block1:
;; @0027                               return
;; }
