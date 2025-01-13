;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut v128))
                    (field (mut v128))))

  (func (param (ref $ty)) (result v128)
    (v128.xor (struct.get $ty 0 (local.get 0))
              (struct.get $ty 0 (local.get 0)))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i8x16 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0022                               trapz v2, user16
;; @0022                               v9 = uextend.i64 v2
;; @0022                               v10 = iconst.i64 16
;; @0022                               v11 = uadd_overflow_trap v9, v10, user1  ; v10 = 16
;;                                     v31 = iconst.i64 48
;; @0022                               v13 = uadd_overflow_trap v9, v31, user1  ; v31 = 48
;; @0022                               v8 = load.i64 notrap aligned readonly v0+48
;; @0022                               v14 = icmp ule v13, v8
;; @0022                               trapz v14, user1
;; @0022                               v6 = load.i64 notrap aligned readonly v0+40
;; @0022                               v15 = iadd v6, v11
;; @0022                               v16 = load.i8x16 notrap aligned little v15
;; @0028                               trapz v2, user16
;; @0028                               trapz v14, user1
;; @0028                               v29 = load.i8x16 notrap aligned little v15
;; @002e                               jump block1
;;
;;                                 block1:
;; @002c                               v30 = bxor.i8x16 v16, v29
;; @002e                               return v30
;; }
