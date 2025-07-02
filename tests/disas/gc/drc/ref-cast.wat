;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $s (struct))
  (func (param anyref) (result (ref $s))
    (ref.cast (ref $s) (local.get 0))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     sig0 = (i64 vmctx, i32, i32) -> i32 tail
;;     fn0 = colocated u1:35 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v36 = stack_addr.i64 ss0
;;                                     store notrap v2, v36
;;                                     v34 = iconst.i32 0
;; @001e                               v4 = icmp eq v2, v34  ; v34 = 0
;; @001e                               v5 = uextend.i32 v4
;; @001e                               brif v5, block4(v34), block2  ; v34 = 0
;;
;;                                 block2:
;; @001e                               v7 = iconst.i32 1
;; @001e                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v37 = iconst.i32 0
;; @001e                               brif v8, block4(v37), block3  ; v37 = 0
;;
;;                                 block3:
;; @001e                               v30 = load.i64 notrap aligned readonly can_move v0+8
;; @001e                               v14 = load.i64 notrap aligned readonly can_move v30+24
;; @001e                               v13 = uextend.i64 v2
;; @001e                               v15 = iadd v14, v13
;; @001e                               v16 = iconst.i64 4
;; @001e                               v17 = iadd v15, v16  ; v16 = 4
;; @001e                               v18 = load.i32 notrap aligned readonly v17
;; @001e                               v11 = load.i64 notrap aligned readonly can_move v0+40
;; @001e                               v12 = load.i32 notrap aligned readonly can_move v11
;; @001e                               v19 = icmp eq v18, v12
;; @001e                               v20 = uextend.i32 v19
;; @001e                               brif v20, block6(v20), block5
;;
;;                                 block5:
;; @001e                               v22 = call fn0(v0, v18, v12), stack_map=[i32 @ ss0+0]
;; @001e                               jump block6(v22)
;;
;;                                 block6(v23: i32):
;; @001e                               jump block4(v23)
;;
;;                                 block4(v24: i32):
;; @001e                               trapz v24, user19
;;                                     v25 = load.i32 notrap v36
;; @0021                               jump block1
;;
;;                                 block1:
;; @0021                               return v25
;; }
