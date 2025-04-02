;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (global $x (mut externref) (ref.null extern))
  (func (export "get") (result externref)
    (global.get $x)
  )
  (func (export "set") (param externref)
    (global.set $x (local.get 0))
  )
)

;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32) -> i64 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v49 = iconst.i64 80
;; @0034                               v4 = iadd v0, v49  ; v49 = 80
;; @0034                               v5 = load.i32 notrap aligned v4
;;                                     v50 = stack_addr.i64 ss0
;;                                     store notrap v5, v50
;;                                     v52 = iconst.i32 1
;; @0034                               v6 = band v5, v52  ; v52 = 1
;;                                     v54 = iconst.i32 0
;; @0034                               v7 = icmp eq v5, v54  ; v54 = 0
;; @0034                               v8 = uextend.i32 v7
;; @0034                               v9 = bor v6, v8
;; @0034                               brif v9, block5, block2
;;
;;                                 block2:
;; @0034                               v11 = load.i64 notrap aligned readonly v0+56
;; @0034                               v12 = load.i64 notrap aligned v11
;; @0034                               v13 = load.i64 notrap aligned v11+8
;; @0034                               v14 = icmp eq v12, v13
;; @0034                               brif v14, block3, block4
;;
;;                                 block4:
;; @0034                               v19 = uextend.i64 v5
;; @0034                               v20 = iconst.i64 8
;; @0034                               v21 = uadd_overflow_trap v19, v20, user1  ; v20 = 8
;; @0034                               v23 = uadd_overflow_trap v21, v20, user1  ; v20 = 8
;; @0034                               v18 = load.i64 notrap aligned readonly can_move v0+48
;; @0034                               v24 = icmp ule v23, v18
;; @0034                               trapz v24, user1
;; @0034                               v16 = load.i64 notrap aligned readonly can_move v0+40
;; @0034                               v25 = iadd v16, v21
;; @0034                               v26 = load.i64 notrap aligned v25
;;                                     v56 = iconst.i64 1
;; @0034                               v27 = iadd v26, v56  ; v56 = 1
;; @0034                               store notrap aligned v27, v25
;;                                     v44 = load.i32 notrap v50
;; @0034                               store notrap aligned v44, v12
;;                                     v59 = iconst.i64 4
;; @0034                               v39 = iadd.i64 v12, v59  ; v59 = 4
;; @0034                               store notrap aligned v39, v11
;; @0034                               jump block5
;;
;;                                 block3 cold:
;; @0034                               v41 = call fn0(v0, v5), stack_map=[i32 @ ss0+0]
;; @0034                               jump block5
;;
;;                                 block5:
;;                                     v42 = load.i32 notrap v50
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v42
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32) tail
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v64 = iconst.i64 80
;; @003b                               v4 = iadd v0, v64  ; v64 = 80
;; @003b                               v5 = load.i32 notrap aligned v4
;;                                     v65 = iconst.i32 1
;; @003b                               v6 = band v2, v65  ; v65 = 1
;;                                     v66 = iconst.i32 0
;; @003b                               v7 = icmp eq v2, v66  ; v66 = 0
;; @003b                               v8 = uextend.i32 v7
;; @003b                               v9 = bor v6, v8
;; @003b                               brif v9, block3, block2
;;
;;                                 block2:
;; @003b                               v14 = uextend.i64 v2
;; @003b                               v43 = iconst.i64 8
;; @003b                               v16 = uadd_overflow_trap v14, v43, user1  ; v43 = 8
;; @003b                               v18 = uadd_overflow_trap v16, v43, user1  ; v43 = 8
;; @003b                               v41 = load.i64 notrap aligned readonly can_move v0+48
;; @003b                               v19 = icmp ule v18, v41
;; @003b                               trapz v19, user1
;; @003b                               v39 = load.i64 notrap aligned readonly can_move v0+40
;; @003b                               v20 = iadd v39, v16
;; @003b                               v21 = load.i64 notrap aligned v20
;;                                     v73 = iconst.i64 1
;; @003b                               v22 = iadd v21, v73  ; v73 = 1
;; @003b                               store notrap aligned v22, v20
;; @003b                               jump block3
;;
;;                                 block3:
;;                                     v85 = iadd.i64 v0, v64  ; v64 = 80
;; @003b                               store.i32 notrap aligned v2, v85
;;                                     v86 = iconst.i32 1
;;                                     v87 = band.i32 v5, v86  ; v86 = 1
;;                                     v88 = iconst.i32 0
;;                                     v89 = icmp.i32 eq v5, v88  ; v88 = 0
;; @003b                               v36 = uextend.i32 v89
;; @003b                               v37 = bor v87, v36
;; @003b                               brif v37, block7, block4
;;
;;                                 block4:
;; @003b                               v42 = uextend.i64 v5
;;                                     v90 = iconst.i64 8
;; @003b                               v44 = uadd_overflow_trap v42, v90, user1  ; v90 = 8
;; @003b                               v46 = uadd_overflow_trap v44, v90, user1  ; v90 = 8
;;                                     v91 = load.i64 notrap aligned readonly can_move v0+48
;; @003b                               v47 = icmp ule v46, v91
;; @003b                               trapz v47, user1
;;                                     v92 = load.i64 notrap aligned readonly can_move v0+40
;; @003b                               v48 = iadd v92, v44
;; @003b                               v49 = load.i64 notrap aligned v48
;;                                     v93 = iconst.i64 1
;;                                     v83 = icmp eq v49, v93  ; v93 = 1
;; @003b                               brif v83, block5, block6
;;
;;                                 block5 cold:
;; @003b                               call fn0(v0, v5)
;; @003b                               jump block7
;;
;;                                 block6:
;;                                     v70 = iconst.i64 -1
;; @003b                               v50 = iadd.i64 v49, v70  ; v70 = -1
;; @003b                               store notrap aligned v50, v48
;; @003b                               jump block7
;;
;;                                 block7:
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
;; }
