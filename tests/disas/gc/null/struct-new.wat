;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (param f32 i32 anyref) (result (ref $ty))
    (struct.new $ty (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, f32, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 32 "VMContext+0x20"
;;     region2 = 2147483648 "GcHeap"
;;     region3 = 40 "VMContext+0x28"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v40 = stack_addr.i64 ss0
;;                                     store notrap v4, v40
;; @002a                               v10 = load.i64 notrap aligned readonly region1 v0+32
;; @002a                               v11 = load.i32 user2 region2 v10
;;                                     v47 = iconst.i32 7
;; @002a                               v14 = uadd_overflow_trap v11, v47, user18  ; v47 = 7
;;                                     v53 = iconst.i32 -8
;; @002a                               v16 = band v14, v53  ; v53 = -8
;; @002a                               v6 = iconst.i32 24
;; @002a                               v17 = uadd_overflow_trap v16, v6, user18  ; v6 = 24
;; @002a                               v19 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002a                               v20 = load.i64 notrap aligned v19+40
;; @002a                               v18 = uextend.i64 v17
;; @002a                               v21 = icmp ule v18, v20
;; @002a                               brif v21, block2, block3
;;
;;                                 block2:
;;                                     v54 = iconst.i32 -1342177256
;; @002a                               v25 = load.i64 notrap aligned readonly can_move v19+32
;;                                     v60 = band.i32 v14, v53  ; v53 = -8
;;                                     v61 = uextend.i64 v60
;; @002a                               v27 = iadd v25, v61
;; @002a                               store user2 region2 v54, v27  ; v54 = -1342177256
;; @002a                               v30 = load.i64 notrap aligned readonly can_move region3 v0+40
;; @002a                               v31 = load.i32 notrap aligned readonly can_move v30
;; @002a                               store user2 region2 v31, v27+4
;; @002a                               store.i32 user2 region2 v17, v10
;; @002a                               v32 = iconst.i64 8
;; @002a                               v33 = iadd v27, v32  ; v32 = 8
;; @002a                               store.f32 user2 little region2 v2, v33
;; @002a                               v34 = iconst.i64 12
;; @002a                               v35 = iadd v27, v34  ; v34 = 12
;; @002a                               istore8.i32 user2 little region2 v3, v35
;;                                     v39 = load.i32 notrap v40
;; @002a                               v36 = iconst.i64 16
;; @002a                               v37 = iadd v27, v36  ; v36 = 16
;; @002a                               store user2 little region2 v39, v37
;; @002d                               jump block1
;;
;;                                 block3 cold:
;; @002a                               v22 = isub.i64 v18, v20
;; @002a                               v23 = call fn0(v0, v22), stack_map=[i32 @ ss0+0]
;; @002a                               jump block2
;;
;;                                 block1:
;;                                     v62 = band.i32 v14, v53  ; v53 = -8
;; @002d                               return v62
;; }
