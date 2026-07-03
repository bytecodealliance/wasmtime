;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (result (ref $ty))
    (struct.new_default $ty)
  )
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 40 "VMContext+0x28"
;;     region3 = 1677721600 "TypeIdsArray+0x0"
;;     region4 = 67108896 "VMStoreContext+0x20"
;;     region5 = 536870912 "GcHeap"
;;     region6 = 67108904 "VMStoreContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v6 = iconst.i32 -1342177280
;; @0021                               v7 = load.i64 notrap aligned readonly can_move region2 v0+40
;; @0021                               v8 = load.i32 notrap aligned readonly can_move region3 v7
;; @0021                               v5 = iconst.i32 40
;; @0021                               v9 = iconst.i32 8
;; @0021                               v10 = call fn0(v0, v6, v8, v5, v9)  ; v6 = -1342177280, v5 = 40, v9 = 8
;; @0021                               v2 = f32const 0.0
;; @0021                               v11 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0021                               v12 = load.i64 notrap aligned readonly can_move region4 v11+32
;; @0021                               v13 = uextend.i64 v10
;; @0021                               v14 = iadd v12, v13
;; @0021                               v15 = iconst.i64 24
;; @0021                               v16 = iadd v14, v15  ; v15 = 24
;; @0021                               store user2 little region5 v2, v16  ; v2 = 0.0
;; @0021                               v3 = iconst.i32 0
;; @0021                               v17 = iconst.i64 28
;; @0021                               v18 = iadd v14, v17  ; v17 = 28
;; @0021                               istore8 user2 little region5 v3, v18  ; v3 = 0
;; @0021                               jump block3
;;
;;                                 block3:
;;                                     v59 = iconst.i32 0
;; @0021                               v19 = iconst.i64 32
;; @0021                               v20 = iadd.i64 v14, v19  ; v19 = 32
;; @0021                               store user2 little region5 v59, v20  ; v59 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v10
;; }
