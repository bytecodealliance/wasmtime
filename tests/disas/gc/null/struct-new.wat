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
;;                                     v38 = stack_addr.i64 ss0
;;                                     store notrap v4, v38
;; @002a                               v10 = load.i64 notrap aligned readonly region0 v0+32
;; @002a                               v11 = load.i32 user2 region1 v10
;;                                     v49 = iconst.i32 7
;; @002a                               v14 = uadd_overflow_trap v11, v49, user18  ; v49 = 7
;;                                     v55 = iconst.i32 -8
;; @002a                               v16 = band v14, v55  ; v55 = -8
;; @002a                               v6 = iconst.i32 24
;; @002a                               v17 = uadd_overflow_trap v16, v6, user18  ; v6 = 24
;; @002a                               v41 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v19 = load.i64 notrap aligned v41+40
;; @002a                               v18 = uextend.i64 v17
;; @002a                               v20 = icmp ule v18, v19
;; @002a                               brif v20, block2, block3
;;
;;                                 block2:
;;                                     v56 = iconst.i32 -1342177256
;; @002a                               v23 = load.i64 notrap aligned readonly can_move v41+32
;;                                     v62 = band.i32 v14, v55  ; v55 = -8
;;                                     v63 = uextend.i64 v62
;; @002a                               v25 = iadd v23, v63
;; @002a                               store user2 region1 v56, v25  ; v56 = -1342177256
;; @002a                               v28 = load.i64 notrap aligned readonly can_move v0+40
;; @002a                               v29 = load.i32 notrap aligned readonly can_move v28
;; @002a                               store user2 region1 v29, v25+4
;; @002a                               store.i32 user2 region1 v17, v10
;; @002a                               v30 = iconst.i64 8
;; @002a                               v31 = iadd v25, v30  ; v30 = 8
;; @002a                               store.f32 user2 little region1 v2, v31
;; @002a                               v32 = iconst.i64 12
;; @002a                               v33 = iadd v25, v32  ; v32 = 12
;; @002a                               istore8.i32 user2 little region1 v3, v33
;;                                     v37 = load.i32 notrap v38
;; @002a                               v34 = iconst.i64 16
;; @002a                               v35 = iadd v25, v34  ; v34 = 16
;; @002a                               store user2 little region1 v37, v35
;; @002d                               jump block1
;;
;;                                 block3 cold:
;; @002a                               v21 = isub.i64 v18, v19
;; @002a                               v22 = call fn0(v0, v21), stack_map=[i32 @ ss0+0]
;; @002a                               jump block2
;;
;;                                 block1:
;;                                     v64 = band.i32 v14, v55  ; v55 = -8
;; @002d                               return v64
;; }
