;;! target = "x86_64"
;;! flags = "-W function-references,gc"
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
;;     region1 = 40 "VMContext+0x28"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v46 = stack_addr.i64 ss0
;;                                     store notrap v4, v46
;; @002a                               v7 = load.i64 notrap aligned readonly can_move v0+32
;; @002a                               v8 = load.i32 notrap aligned v7
;; @002a                               v9 = load.i32 notrap aligned v7+4
;; @002a                               v15 = uextend.i64 v8
;;                                     v47 = iconst.i64 32
;; @002a                               v16 = iadd v15, v47  ; v47 = 32
;; @002a                               v17 = uextend.i64 v9
;; @002a                               v18 = icmp ule v16, v17
;; @002a                               brif v18, block2, block3
;;
;;                                 block2:
;;                                     v63 = iconst.i32 32
;;                                     v61 = iadd.i32 v8, v63  ; v63 = 32
;; @002a                               store notrap aligned region2 v61, v7
;;                                     v64 = iconst.i32 -1342177246
;;                                     v65 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v66 = load.i64 notrap aligned readonly can_move v65+32
;; @002a                               v32 = iadd v66, v15
;; @002a                               store notrap aligned v64, v32  ; v64 = -1342177246
;;                                     v67 = load.i64 notrap aligned readonly can_move region1 v0+40
;;                                     v68 = load.i32 notrap aligned readonly can_move v67
;; @002a                               store notrap aligned v68, v32+4
;;                                     v69 = iconst.i64 32
;; @002a                               istore32 notrap aligned v69, v32+8  ; v69 = 32
;; @002a                               jump block4(v8, v32)
;;
;;                                 block3 cold:
;; @002a                               v19 = iconst.i32 -1342177246
;; @002a                               v20 = load.i64 notrap aligned readonly can_move region1 v0+40
;; @002a                               v21 = load.i32 notrap aligned readonly can_move v20
;; @002a                               v6 = iconst.i32 32
;; @002a                               v22 = iconst.i32 16
;; @002a                               v23 = call fn0(v0, v19, v21, v6, v22), stack_map=[i32 @ ss0+0]  ; v19 = -1342177246, v6 = 32, v22 = 16
;; @002a                               v24 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002a                               v25 = load.i64 notrap aligned readonly can_move v24+32
;; @002a                               v26 = uextend.i64 v23
;; @002a                               v27 = iadd v25, v26
;; @002a                               jump block4(v23, v27)
;;
;;                                 block4(v36: i32, v37: i64):
;; @002a                               v38 = iconst.i64 16
;; @002a                               v39 = iadd v37, v38  ; v38 = 16
;; @002a                               store.f32 user2 little region3 v2, v39
;; @002a                               v40 = iconst.i64 20
;; @002a                               v41 = iadd v37, v40  ; v40 = 20
;; @002a                               istore8.i32 user2 little region3 v3, v41
;;                                     v45 = load.i32 notrap v46
;; @002a                               v42 = iconst.i64 24
;; @002a                               v43 = iadd v37, v42  ; v42 = 24
;; @002a                               store user2 little region3 v45, v43
;; @002d                               jump block1(v36)
;;
;;                                 block1(v5: i32):
;; @002d                               return v5
;; }
