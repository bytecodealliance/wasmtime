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
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext) -> i32 uext tail
;;     fn0 = colocated u1:35 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v39 = stack_addr.i64 ss0
;;                                     store notrap v2, v39
;;                                     v41 = iconst.i32 0
;; @001e                               v4 = icmp eq v2, v41  ; v41 = 0
;; @001e                               v5 = uextend.i32 v4
;; @001e                               v7 = iconst.i32 1
;;                                     v49 = select v2, v7, v41  ; v7 = 1, v41 = 0
;; @001e                               brif v5, block4(v49), block2
;;
;;                                 block2:
;;                                     v56 = iconst.i32 1
;;                                     v57 = band.i32 v2, v56  ; v56 = 1
;;                                     v58 = iconst.i32 0
;;                                     v59 = select v57, v58, v56  ; v58 = 0, v56 = 1
;; @001e                               brif v57, block4(v59), block3
;;
;;                                 block3:
;; @001e                               v21 = uextend.i64 v2
;; @001e                               v22 = iconst.i64 4
;; @001e                               v23 = uadd_overflow_trap v21, v22, user1  ; v22 = 4
;; @001e                               v24 = iconst.i64 8
;; @001e                               v25 = uadd_overflow_trap v23, v24, user1  ; v24 = 8
;; @001e                               v20 = load.i64 notrap aligned readonly v0+48
;; @001e                               v26 = icmp ule v25, v20
;; @001e                               trapz v26, user1
;; @001e                               v18 = load.i64 notrap aligned readonly v0+40
;; @001e                               v27 = iadd v18, v23
;; @001e                               v28 = load.i32 notrap aligned readonly v27
;; @001e                               v15 = load.i64 notrap aligned readonly v0+80
;; @001e                               v16 = load.i32 notrap aligned readonly v15
;; @001e                               v29 = icmp eq v28, v16
;; @001e                               v30 = uextend.i32 v29
;; @001e                               brif v30, block6(v30), block5
;;
;;                                 block5:
;; @001e                               v32 = call fn0(v0, v28, v16), stack_map=[i32 @ ss0+0]
;; @001e                               jump block6(v32)
;;
;;                                 block6(v33: i32):
;; @001e                               jump block4(v33)
;;
;;                                 block4(v34: i32):
;; @001e                               trapz v34, user19
;;                                     v35 = load.i32 notrap v39
;; @0021                               jump block1
;;
;;                                 block1:
;; @0021                               return v35
;; }
