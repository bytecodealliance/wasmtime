;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (param (ref null $ty) f32)
    (struct.set $ty 0 (local.get 0) (local.get 1))
  )

  (func (param (ref null $ty) i32)
    (struct.set $ty 1 (local.get 0) (local.get 1))
  )

  (func (param (ref null $ty) anyref)
    (struct.set $ty 2 (local.get 0) (local.get 1))
  )
)
;; function u0:0(i64 vmctx, i64, i32, f32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: f32):
;; @0034                               trapz v2, user16
;; @0034                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0034                               v6 = load.i64 notrap aligned readonly can_move region2 v5+32
;; @0034                               v4 = uextend.i64 v2
;; @0034                               v7 = iadd v6, v4
;; @0034                               v8 = iconst.i64 8
;; @0034                               v9 = iadd v7, v8  ; v8 = 8
;; @0034                               store user2 little region4 v3, v9
;; @0038                               jump block1
;;
;;                                 block1:
;; @0038                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003f                               trapz v2, user16
;; @003f                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003f                               v6 = load.i64 notrap aligned readonly can_move region2 v5+32
;; @003f                               v4 = uextend.i64 v2
;; @003f                               v7 = iadd v6, v4
;; @003f                               v8 = iconst.i64 12
;; @003f                               v9 = iadd v7, v8  ; v8 = 12
;; @003f                               istore8 user2 little region4 v3, v9
;; @0043                               jump block1
;;
;;                                 block1:
;; @0043                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 268435488 "VMStoreContext+0x20"
;;     region3 = 268435496 "VMStoreContext+0x28"
;;     region4 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @004a                               trapz v2, user16
;; @004a                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004a                               v6 = load.i64 notrap aligned readonly can_move region2 v5+32
;; @004a                               v4 = uextend.i64 v2
;; @004a                               v7 = iadd v6, v4
;; @004a                               v8 = iconst.i64 16
;; @004a                               v9 = iadd v7, v8  ; v8 = 16
;; @004a                               store user2 little region4 v3, v9
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return
;; }
