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
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v10 = load.i64 notrap aligned readonly region0 v0+32
;; @0021                               v11 = load.i32 user2 region1 v10
;;                                     v45 = iconst.i32 7
;; @0021                               v14 = uadd_overflow_trap v11, v45, user18  ; v45 = 7
;;                                     v51 = iconst.i32 -8
;; @0021                               v16 = band v14, v51  ; v51 = -8
;; @0021                               v6 = iconst.i32 24
;; @0021                               v17 = uadd_overflow_trap v16, v6, user18  ; v6 = 24
;; @0021                               v38 = load.i64 notrap aligned readonly can_move v0+8
;; @0021                               v19 = load.i64 notrap aligned v38+40
;; @0021                               v18 = uextend.i64 v17
;; @0021                               v20 = icmp ule v18, v19
;; @0021                               brif v20, block2, block3
;;
;;                                 block2:
;;                                     v52 = iconst.i32 -1342177256
;; @0021                               v23 = load.i64 notrap aligned readonly can_move v38+32
;;                                     v58 = band.i32 v14, v51  ; v51 = -8
;;                                     v59 = uextend.i64 v58
;; @0021                               v25 = iadd v23, v59
;; @0021                               store user2 region1 v52, v25  ; v52 = -1342177256
;; @0021                               v28 = load.i64 notrap aligned readonly can_move v0+40
;; @0021                               v29 = load.i32 notrap aligned readonly can_move v28
;; @0021                               store user2 region1 v29, v25+4
;; @0021                               store.i32 user2 region1 v17, v10
;; @0021                               v3 = f32const 0.0
;; @0021                               v30 = iconst.i64 8
;; @0021                               v31 = iadd v25, v30  ; v30 = 8
;; @0021                               store user2 little region1 v3, v31  ; v3 = 0.0
;; @0021                               v4 = iconst.i32 0
;; @0021                               v32 = iconst.i64 12
;; @0021                               v33 = iadd v25, v32  ; v32 = 12
;; @0021                               istore8 user2 little region1 v4, v33  ; v4 = 0
;; @0021                               v34 = iconst.i64 16
;; @0021                               v35 = iadd v25, v34  ; v34 = 16
;; @0021                               store user2 little region1 v4, v35  ; v4 = 0
;; @0024                               jump block1
;;
;;                                 block3 cold:
;; @0021                               v21 = isub.i64 v18, v19
;; @0021                               v22 = call fn0(v0, v21)
;; @0021                               jump block2
;;
;;                                 block1:
;;                                     v60 = band.i32 v14, v51  ; v51 = -8
;; @0024                               return v60
;; }
