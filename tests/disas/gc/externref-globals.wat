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
;;                                     v43 = iconst.i64 96
;; @0034                               v4 = iadd v0, v43  ; v43 = 96
;; @0034                               v5 = load.i32 notrap aligned v4
;;                                     v44 = stack_addr.i64 ss0
;;                                     store notrap v5, v44
;;                                     v46 = iconst.i32 0
;; @0034                               v6 = icmp eq v5, v46  ; v46 = 0
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
;; @0034                               v15 = uextend.i64 v5
;; @0034                               v16 = iconst.i64 8
;; @0034                               v17 = uadd_overflow_trap v15, v16, user65535  ; v16 = 8
;; @0034                               v19 = uadd_overflow_trap v17, v16, user65535  ; v16 = 8
;; @0034                               v14 = load.i64 notrap aligned readonly v0+48
;; @0034                               v20 = icmp ule v19, v14
;; @0034                               trapz v20, user65535
;; @0034                               v13 = load.i64 notrap aligned readonly v0+40
;; @0034                               v21 = iadd v13, v17
;; @0034                               v22 = load.i64 notrap aligned v21
;;                                     v40 = load.i32 notrap v44
;; @0034                               v27 = uextend.i64 v40
;; @0034                               v29 = uadd_overflow_trap v27, v16, user65535  ; v16 = 8
;; @0034                               v31 = uadd_overflow_trap v29, v16, user65535  ; v16 = 8
;; @0034                               v32 = icmp ule v31, v14
;; @0034                               trapz v32, user65535
;;                                     v48 = iconst.i64 1
;; @0034                               v23 = iadd v22, v48  ; v48 = 1
;; @0034                               v33 = iadd v13, v29
;; @0034                               store notrap aligned v23, v33
;;                                     v39 = load.i32 notrap v44
;; @0034                               store notrap aligned v39, v9
;;                                     v51 = iconst.i64 4
;; @0034                               v34 = iadd.i64 v9, v51  ; v51 = 4
;; @0034                               store notrap aligned v34, v8
;; @0034                               jump block5
;;
;;                                 block3 cold:
;; @0034                               v36 = call fn0(v0, v5), stack_map=[i32 @ ss0+0]
;; @0034                               jump block5
;;
;;                                 block5:
;;                                     v37 = load.i32 notrap v44
;; @0036                               jump block1
;;
;;                                 block1:
;; @0036                               return v37
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
;;                                     v54 = iconst.i64 96
;; @003b                               v4 = iadd v0, v54  ; v54 = 96
;; @003b                               v5 = load.i32 notrap aligned v4
;;                                     v55 = iconst.i32 0
;; @003b                               v6 = icmp eq v2, v55  ; v55 = 0
;; @003b                               brif v6, block3, block2
;;
;;                                 block2:
;; @003b                               v10 = uextend.i64 v2
;; @003b                               v34 = iconst.i64 8
;; @003b                               v12 = uadd_overflow_trap v10, v34, user65535  ; v34 = 8
;; @003b                               v14 = uadd_overflow_trap v12, v34, user65535  ; v34 = 8
;; @003b                               v32 = load.i64 notrap aligned readonly v0+48
;; @003b                               v15 = icmp ule v14, v32
;; @003b                               trapz v15, user65535
;; @003b                               v31 = load.i64 notrap aligned readonly v0+40
;; @003b                               v16 = iadd v31, v12
;; @003b                               v17 = load.i64 notrap aligned v16
;; @003b                               trapz v15, user65535
;;                                     v56 = iconst.i64 1
;; @003b                               v18 = iadd v17, v56  ; v56 = 1
;; @003b                               store notrap aligned v18, v16
;; @003b                               jump block3
;;
;;                                 block3:
;;                                     v60 = iadd.i64 v0, v54  ; v54 = 96
;; @003b                               store.i32 notrap aligned v2, v60
;;                                     v61 = iconst.i32 0
;;                                     v62 = icmp.i32 eq v5, v61  ; v61 = 0
;; @003b                               brif v62, block7, block4
;;
;;                                 block4:
;; @003b                               v33 = uextend.i64 v5
;;                                     v63 = iconst.i64 8
;; @003b                               v35 = uadd_overflow_trap v33, v63, user65535  ; v63 = 8
;; @003b                               v37 = uadd_overflow_trap v35, v63, user65535  ; v63 = 8
;;                                     v64 = load.i64 notrap aligned readonly v0+48
;; @003b                               v38 = icmp ule v37, v64
;; @003b                               trapz v38, user65535
;;                                     v65 = load.i64 notrap aligned readonly v0+40
;; @003b                               v39 = iadd v65, v35
;; @003b                               v40 = load.i64 notrap aligned v39
;;                                     v58 = iconst.i64 -1
;; @003b                               v41 = iadd v40, v58  ; v58 = -1
;;                                     v59 = iconst.i64 0
;; @003b                               v42 = icmp eq v41, v59  ; v59 = 0
;; @003b                               brif v42, block5, block6
;;
;;                                 block5 cold:
;; @003b                               call fn0(v0, v5)
;; @003b                               jump block7
;;
;;                                 block6:
;; @003b                               trapz.i8 v38, user65535
;;                                     v66 = iadd.i64 v40, v58  ; v58 = -1
;; @003b                               store notrap aligned v66, v39
;; @003b                               jump block7
;;
;;                                 block7:
;; @003d                               jump block1
;;
;;                                 block1:
;; @003d                               return
;; }
