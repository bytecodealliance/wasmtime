;;! target = "x86_64"
;;! flags = "-W function-references,gc"
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
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32) -> i32 system_v
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v45 = iconst.i64 96
;; @0034                               v4 = iadd v0, v45  ; v45 = 96
;; @0034                               v5 = load.i32 notrap aligned v4
;;                                     v46 = stack_addr.i64 ss0
;;                                     store notrap v5, v46
;;                                     v48 = iconst.i32 0
;; @0034                               v6 = icmp eq v5, v48  ; v48 = 0
;; @0034                               brif v6, block5, block2
;;
;;                                 block2:
;; @0034                               v8 = load.i64 notrap aligned v0+56
;; @0034                               v9 = load.i64 notrap aligned v8
;; @0034                               v10 = load.i64 notrap aligned v8+8
;; @0034                               v11 = icmp eq v9, v10
;; @0034                               brif v11, block3, block4
;;
;;                                 block4:
;; @0034                               v16 = uextend.i64 v5
;; @0034                               v17 = iconst.i64 8
;; @0034                               v18 = uadd_overflow_trap v16, v17, user1  ; v17 = 8
;; @0034                               v20 = uadd_overflow_trap v18, v17, user1  ; v17 = 8
;; @0034                               v15 = load.i64 notrap aligned readonly v0+48
;; @0034                               v21 = icmp ule v20, v15
;; @0034                               trapz v21, user1
;; @0034                               v13 = load.i64 notrap aligned readonly v0+40
;; @0034                               v22 = iadd v13, v18
;; @0034                               v23 = load.i64 notrap aligned v22
;;                                     v42 = load.i32 notrap v46
;; @0034                               v29 = uextend.i64 v42
;; @0034                               v31 = uadd_overflow_trap v29, v17, user1  ; v17 = 8
;; @0034                               v33 = uadd_overflow_trap v31, v17, user1  ; v17 = 8
;; @0034                               v34 = icmp ule v33, v15
;; @0034                               trapz v34, user1
;;                                     v50 = iconst.i64 1
;; @0034                               v24 = iadd v23, v50  ; v50 = 1
;; @0034                               v35 = iadd v13, v31
;; @0034                               store notrap aligned v24, v35
;;                                     v41 = load.i32 notrap v46
;; @0034                               store notrap aligned v41, v9
;;                                     v53 = iconst.i64 4
;; @0034                               v36 = iadd.i64 v9, v53  ; v53 = 4
;; @0034                               store notrap aligned v36, v8
;; @0034                               jump block5
;;
;;                                 block3 cold:
;; @0034                               v38 = call fn0(v0, v5), stack_map=[i32 @ ss0+0]
;; @0034                               jump block5
;;
;;                                 block5:
;;                                     v39 = load.i32 notrap v46
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v39
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext) system_v
;;     fn0 = colocated u1:25 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v58 = iconst.i64 96
;; @003b                               v4 = iadd v0, v58  ; v58 = 96
;; @003b                               v5 = load.i32 notrap aligned v4
;;                                     v59 = iconst.i32 0
;; @003b                               v6 = icmp eq v2, v59  ; v59 = 0
;; @003b                               brif v6, block3, block2
;;
;;                                 block2:
;; @003b                               v11 = uextend.i64 v2
;; @003b                               v37 = iconst.i64 8
;; @003b                               v13 = uadd_overflow_trap v11, v37, user1  ; v37 = 8
;; @003b                               v15 = uadd_overflow_trap v13, v37, user1  ; v37 = 8
;; @003b                               v35 = load.i64 notrap aligned readonly v0+48
;; @003b                               v16 = icmp ule v15, v35
;; @003b                               trapz v16, user1
;; @003b                               v33 = load.i64 notrap aligned readonly v0+40
;; @003b                               v17 = iadd v33, v13
;; @003b                               v18 = load.i64 notrap aligned v17
;; @003b                               trapz v16, user1
;;                                     v60 = iconst.i64 1
;; @003b                               v19 = iadd v18, v60  ; v60 = 1
;; @003b                               store notrap aligned v19, v17
;; @003b                               jump block3
;;
;;                                 block3:
;;                                     v64 = iadd.i64 v0, v58  ; v58 = 96
;; @003b                               store.i32 notrap aligned v2, v64
;;                                     v65 = iconst.i32 0
;;                                     v66 = icmp.i32 eq v5, v65  ; v65 = 0
;; @003b                               brif v66, block7, block4
;;
;;                                 block4:
;; @003b                               v36 = uextend.i64 v5
;;                                     v67 = iconst.i64 8
;; @003b                               v38 = uadd_overflow_trap v36, v67, user1  ; v67 = 8
;; @003b                               v40 = uadd_overflow_trap v38, v67, user1  ; v67 = 8
;;                                     v68 = load.i64 notrap aligned readonly v0+48
;; @003b                               v41 = icmp ule v40, v68
;; @003b                               trapz v41, user1
;;                                     v69 = load.i64 notrap aligned readonly v0+40
;; @003b                               v42 = iadd v69, v38
;; @003b                               v43 = load.i64 notrap aligned v42
;;                                     v62 = iconst.i64 -1
;; @003b                               v44 = iadd v43, v62  ; v62 = -1
;;                                     v63 = iconst.i64 0
;; @003b                               v45 = icmp eq v44, v63  ; v63 = 0
;; @003b                               brif v45, block5, block6
;;
;;                                 block5 cold:
;; @003b                               call fn0(v0, v5)
;; @003b                               jump block7
;;
;;                                 block6:
;; @003b                               trapz.i8 v41, user1
;;                                     v70 = iadd.i64 v43, v62  ; v62 = -1
;; @003b                               store notrap aligned v70, v42
;; @003b                               jump block7
;;
;;                                 block7:
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
;; }
