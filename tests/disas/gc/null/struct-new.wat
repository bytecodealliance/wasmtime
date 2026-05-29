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
;;     gv6 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u805306368:23 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v45 = stack_addr.i64 ss0
;;                                     store notrap v4, v45
;; @002a                               v11 = load.i64 notrap aligned readonly region0 v0+32
;; @002a                               v12 = load.i32 user2 region1 v11
;;                                     v52 = iconst.i32 7
;; @002a                               v15 = uadd_overflow_trap v12, v52, user18  ; v52 = 7
;;                                     v58 = iconst.i32 -8
;; @002a                               v17 = band v15, v58  ; v58 = -8
;; @002a                               v6 = iconst.i32 24
;; @002a                               v18 = uadd_overflow_trap v17, v6, user18  ; v6 = 24
;; @002a                               v43 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v20 = load.i64 notrap aligned v43+40
;; @002a                               v19 = uextend.i64 v18
;; @002a                               v21 = icmp ule v19, v20
;; @002a                               brif v21, block2, block3
;;
;;                                 block2:
;;                                     v59 = iconst.i32 -1342177256
;; @002a                               v25 = load.i64 notrap aligned readonly can_move v43+32
;;                                     v65 = band.i32 v15, v58  ; v58 = -8
;;                                     v66 = uextend.i64 v65
;; @002a                               v27 = iadd v25, v66
;; @002a                               store user2 region1 v59, v27  ; v59 = -1342177256
;; @002a                               v31 = load.i64 notrap aligned readonly can_move v0+40
;; @002a                               v32 = load.i32 notrap aligned readonly can_move v31
;; @002a                               store user2 region1 v32, v27+4
;; @002a                               store.i32 user2 region1 v18, v11
;; @002a                               v33 = iconst.i64 8
;; @002a                               v34 = iadd v27, v33  ; v33 = 8
;; @002a                               store.f32 user2 little region1 v2, v34
;; @002a                               v35 = iconst.i64 12
;; @002a                               v36 = iadd v27, v35  ; v35 = 12
;; @002a                               istore8.i32 user2 little region1 v3, v36
;;                                     v39 = load.i32 notrap v45
;; @002a                               v37 = iconst.i64 16
;; @002a                               v38 = iadd v27, v37  ; v37 = 16
;; @002a                               store user2 little region1 v39, v38
;; @002d                               jump block1
;;
;;                                 block3 cold:
;; @002a                               v23 = isub.i64 v19, v20
;; @002a                               v24 = call fn0(v0, v23), stack_map=[i32 @ ss0+0]
;; @002a                               jump block2
;;
;;                                 block1:
;;                                     v67 = band.i32 v15, v58  ; v58 = -8
;; @002d                               return v67
;; }
