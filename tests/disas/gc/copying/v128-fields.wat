;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
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
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0022                               trapz v2, user16
;; @0022                               v4 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0022                               v5 = load.i64 notrap aligned readonly can_move region2 v4+32
;; @0022                               v3 = uextend.i64 v2
;; @0022                               v6 = iadd v5, v3
;; @0022                               v7 = iconst.i64 16
;; @0022                               v8 = iadd v6, v7  ; v7 = 16
;; @0022                               v9 = load.i8x16 user2 little region4 v8
;; @002e                               jump block1
;;
;;                                 block1:
;; @002c                               v17 = bxor.i8x16 v9, v9
;; @002e                               return v17
;; }
