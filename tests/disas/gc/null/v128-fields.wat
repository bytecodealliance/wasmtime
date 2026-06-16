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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0022                               trapz v2, user16
;; @0022                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v6 = load.i64 notrap aligned readonly can_move v5+32
;; @0022                               v4 = uextend.i64 v2
;; @0022                               v7 = iadd v6, v4
;; @0022                               v8 = iconst.i64 16
;; @0022                               v9 = iadd v7, v8  ; v8 = 16
;; @0022                               v10 = load.i8x16 user2 little region1 v9
;; @002e                               jump block1
;;
;;                                 block1:
;; @002c                               v18 = bxor.i8x16 v10, v10
;; @002e                               return v18
;; }
