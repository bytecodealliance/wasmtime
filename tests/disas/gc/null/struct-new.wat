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
;;     region0 = 32 "VMContext+0x20"
;;     region1 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v39 = stack_addr.i64 ss0
;;                                     store notrap v4, v39
;; @002a                               v10 = load.i64 notrap aligned readonly region0 v0+32
;; @002a                               v11 = load.i32 user2 region1 v10
;;                                     v48 = iconst.i32 7
;; @002a                               v14 = uadd_overflow_trap v11, v48, user18  ; v48 = 7
;;                                     v54 = iconst.i32 -8
;; @002a                               v16 = band v14, v54  ; v54 = -8
;; @002a                               v6 = iconst.i32 24
;; @002a                               v17 = uadd_overflow_trap v16, v6, user18  ; v6 = 24
;; @002a                               v40 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v19 = load.i64 notrap aligned v40+40
;; @002a                               v18 = uextend.i64 v17
;; @002a                               v20 = icmp ule v18, v19
;; @002a                               brif v20, block2, block3
;;
;;                                 block2:
;;                                     v55 = iconst.i32 -1342177256
;; @002a                               v24 = load.i64 notrap aligned readonly can_move v40+32
;;                                     v61 = band.i32 v14, v54  ; v54 = -8
;;                                     v62 = uextend.i64 v61
;; @002a                               v26 = iadd v24, v62
;; @002a                               store user2 region1 v55, v26  ; v55 = -1342177256
;; @002a                               v29 = load.i64 notrap aligned readonly can_move v0+40
;; @002a                               v30 = load.i32 notrap aligned readonly can_move v29
;; @002a                               store user2 region1 v30, v26+4
;; @002a                               store.i32 user2 region1 v17, v10
;; @002a                               v31 = iconst.i64 8
;; @002a                               v32 = iadd v26, v31  ; v31 = 8
;; @002a                               store.f32 user2 little region1 v2, v32
;; @002a                               v33 = iconst.i64 12
;; @002a                               v34 = iadd v26, v33  ; v33 = 12
;; @002a                               istore8.i32 user2 little region1 v3, v34
;;                                     v38 = load.i32 notrap v39
;; @002a                               v35 = iconst.i64 16
;; @002a                               v36 = iadd v26, v35  ; v35 = 16
;; @002a                               store user2 little region1 v38, v36
;; @002d                               jump block1
;;
;;                                 block3 cold:
;; @002a                               v21 = isub.i64 v18, v19
;; @002a                               v22 = call fn0(v0, v21), stack_map=[i32 @ ss0+0]
;; @002a                               jump block2
;;
;;                                 block1:
;;                                     v63 = band.i32 v14, v54  ; v54 = -8
;; @002d                               return v63
;; }
