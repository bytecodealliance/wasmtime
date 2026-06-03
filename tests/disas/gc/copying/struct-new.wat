;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
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
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v44 = stack_addr.i64 ss0
;;                                     store notrap v4, v44
;; @002a                               v7 = load.i64 notrap aligned readonly can_move v0+32
;; @002a                               v8 = load.i32 notrap aligned v7
;; @002a                               v9 = load.i32 notrap aligned v7+4
;; @002a                               v15 = uextend.i64 v8
;;                                     v49 = iconst.i64 32
;; @002a                               v16 = iadd v15, v49  ; v49 = 32
;; @002a                               v17 = uextend.i64 v9
;; @002a                               v18 = icmp ule v16, v17
;; @002a                               brif v18, block2, block3
;;
;;                                 block2:
;;                                     v65 = iconst.i32 32
;;                                     v63 = iadd.i32 v8, v65  ; v65 = 32
;; @002a                               store notrap aligned region0 v63, v7
;;                                     v66 = iconst.i32 -1342177246
;;                                     v67 = load.i64 notrap aligned readonly can_move v0+8
;;                                     v68 = load.i64 notrap aligned readonly can_move v67+32
;; @002a                               v30 = iadd v68, v15
;; @002a                               store notrap aligned v66, v30  ; v66 = -1342177246
;;                                     v69 = load.i64 notrap aligned readonly can_move v0+40
;;                                     v70 = load.i32 notrap aligned readonly can_move v69
;; @002a                               store notrap aligned v70, v30+4
;;                                     v71 = iconst.i64 32
;; @002a                               istore32 notrap aligned v71, v30+8  ; v71 = 32
;; @002a                               jump block4(v8, v30)
;;
;;                                 block3 cold:
;; @002a                               v19 = iconst.i32 -1342177246
;; @002a                               v20 = load.i64 notrap aligned readonly can_move v0+40
;; @002a                               v21 = load.i32 notrap aligned readonly can_move v20
;; @002a                               v6 = iconst.i32 32
;; @002a                               v22 = iconst.i32 16
;; @002a                               v23 = call fn0(v0, v19, v21, v6, v22), stack_map=[i32 @ ss0+0]  ; v19 = -1342177246, v6 = 32, v22 = 16
;; @002a                               v45 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v24 = load.i64 notrap aligned readonly can_move v45+32
;; @002a                               v25 = uextend.i64 v23
;; @002a                               v26 = iadd v24, v25
;; @002a                               jump block4(v23, v26)
;;
;;                                 block4(v34: i32, v35: i64):
;; @002a                               v36 = iconst.i64 16
;; @002a                               v37 = iadd v35, v36  ; v36 = 16
;; @002a                               store.f32 user2 little region1 v2, v37
;; @002a                               v38 = iconst.i64 20
;; @002a                               v39 = iadd v35, v38  ; v38 = 20
;; @002a                               istore8.i32 user2 little region1 v3, v39
;;                                     v43 = load.i32 notrap v44
;; @002a                               v40 = iconst.i64 24
;; @002a                               v41 = iadd v35, v40  ; v40 = 24
;; @002a                               store user2 little region1 v43, v41
;; @002d                               jump block1(v34)
;;
;;                                 block1(v5: i32):
;; @002d                               return v5
;; }
