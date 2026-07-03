;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
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
;; @0034                               v8 = iconst.i64 24
;; @0034                               v9 = iadd v7, v8  ; v8 = 24
;; @0034                               store user2 little region4 v3, v9
;; @0038                               jump block1
;;
;;                                 block1:
;; @0038                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
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
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @003f                               trapz v2, user16
;; @003f                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @003f                               v6 = load.i64 notrap aligned readonly can_move region2 v5+32
;; @003f                               v4 = uextend.i64 v2
;; @003f                               v7 = iadd v6, v4
;; @003f                               v8 = iconst.i64 28
;; @003f                               v9 = iadd v7, v8  ; v8 = 28
;; @003f                               istore8 user2 little region4 v3, v9
;; @0043                               jump block1
;;
;;                                 block1:
;; @0043                               return
;; }
;;
;; function u0:2(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 67108896 "VMStoreContext+0x20"
;;     region3 = 67108904 "VMStoreContext+0x28"
;;     region4 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u805306368:22 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @004a                               trapz v2, user16
;; @004a                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @004a                               v6 = load.i64 notrap aligned readonly can_move region2 v5+32
;; @004a                               v4 = uextend.i64 v2
;; @004a                               v7 = iadd v6, v4
;; @004a                               v8 = iconst.i64 32
;; @004a                               v9 = iadd v7, v8  ; v8 = 32
;; @004a                               v10 = load.i32 user2 little region4 v9
;; @004a                               v11 = iconst.i32 1
;; @004a                               v12 = band v3, v11  ; v11 = 1
;; @004a                               v13 = iconst.i32 0
;; @004a                               v14 = icmp eq v3, v13  ; v13 = 0
;; @004a                               v15 = uextend.i32 v14
;; @004a                               v16 = bor v12, v15
;; @004a                               brif v16, block3, block2
;;
;;                                 block2:
;; @004a                               v17 = uextend.i64 v3
;; @004a                               v20 = iadd.i64 v6, v17
;; @004a                               v21 = iconst.i64 8
;; @004a                               v22 = iadd v20, v21  ; v21 = 8
;; @004a                               v23 = load.i64 user2 region4 v22
;; @004a                               v24 = iconst.i64 1
;; @004a                               v25 = iadd v23, v24  ; v24 = 1
;; @004a                               store user2 region4 v25, v22
;; @004a                               jump block3
;;
;;                                 block3:
;;                                     v67 = iadd.i64 v7, v8  ; v8 = 32
;; @004a                               store.i32 user2 little region4 v3, v67
;;                                     v68 = iconst.i32 1
;;                                     v69 = band.i32 v10, v68  ; v68 = 1
;;                                     v70 = iconst.i32 0
;;                                     v71 = icmp.i32 eq v10, v70  ; v70 = 0
;; @004a                               v36 = uextend.i32 v71
;; @004a                               v37 = bor v69, v36
;; @004a                               brif v37, block7, block4
;;
;;                                 block4:
;; @004a                               v38 = uextend.i64 v10
;; @004a                               v41 = iadd.i64 v6, v38
;;                                     v72 = iconst.i64 8
;; @004a                               v43 = iadd v41, v72  ; v72 = 8
;; @004a                               v44 = load.i64 user2 region4 v43
;;                                     v73 = iconst.i64 1
;;                                     v65 = icmp eq v44, v73  ; v73 = 1
;; @004a                               brif v65, block5, block6
;;
;;                                 block5 cold:
;; @004a                               call fn0(v0, v10)
;; @004a                               jump block7
;;
;;                                 block6:
;; @004a                               v45 = iconst.i64 -1
;; @004a                               v46 = iadd.i64 v44, v45  ; v45 = -1
;;                                     v74 = iadd.i64 v41, v72  ; v72 = 8
;; @004a                               store user2 region4 v46, v74
;; @004a                               jump block7
;;
;;                                 block7:
;; @004e                               jump block1
;;
;;                                 block1:
;; @004e                               return
;; }
