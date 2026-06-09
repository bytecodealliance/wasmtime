;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
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
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v10 = load.i64 notrap aligned readonly region1 v0+32
;; @0021                               v11 = load.i32 user2 region2 v10
;;                                     v43 = iconst.i32 7
;; @0021                               v14 = uadd_overflow_trap v11, v43, user18  ; v43 = 7
;;                                     v49 = iconst.i32 -8
;; @0021                               v16 = band v14, v49  ; v49 = -8
;; @0021                               v6 = iconst.i32 24
;; @0021                               v17 = uadd_overflow_trap v16, v6, user18  ; v6 = 24
;; @0021                               v19 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0021                               v20 = load.i64 notrap aligned v19+40
;; @0021                               v18 = uextend.i64 v17
;; @0021                               v21 = icmp ule v18, v20
;; @0021                               brif v21, block2, block3
;;
;;                                 block2:
;;                                     v50 = iconst.i32 -1342177256
;; @0021                               v25 = load.i64 notrap aligned readonly can_move v19+32
;;                                     v56 = band.i32 v14, v49  ; v49 = -8
;;                                     v57 = uextend.i64 v56
;; @0021                               v27 = iadd v25, v57
;; @0021                               store user2 region2 v50, v27  ; v50 = -1342177256
;; @0021                               v30 = load.i64 notrap aligned readonly can_move region3 v0+40
;; @0021                               v31 = load.i32 notrap aligned readonly can_move v30
;; @0021                               store user2 region2 v31, v27+4
;; @0021                               store.i32 user2 region2 v17, v10
;; @0021                               v3 = f32const 0.0
;; @0021                               v32 = iconst.i64 8
;; @0021                               v33 = iadd v27, v32  ; v32 = 8
;; @0021                               store user2 little region2 v3, v33  ; v3 = 0.0
;; @0021                               v4 = iconst.i32 0
;; @0021                               v34 = iconst.i64 12
;; @0021                               v35 = iadd v27, v34  ; v34 = 12
;; @0021                               istore8 user2 little region2 v4, v35  ; v4 = 0
;; @0021                               v36 = iconst.i64 16
;; @0021                               v37 = iadd v27, v36  ; v36 = 16
;; @0021                               store user2 little region2 v4, v37  ; v4 = 0
;; @0024                               jump block1
;;
;;                                 block3 cold:
;; @0021                               v22 = isub.i64 v18, v20
;; @0021                               v23 = call fn0(v0, v22)
;; @0021                               jump block2
;;
;;                                 block1:
;;                                     v58 = band.i32 v14, v49  ; v49 = -8
;; @0024                               return v58
;; }
