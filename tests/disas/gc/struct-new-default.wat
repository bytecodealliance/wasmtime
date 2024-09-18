;;! target = "x86_64"
;;! flags = "-W function-references,gc"
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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i32 uext, i32 uext) -> i32 system_v
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v7 = iconst.i32 -1476395008
;; @0021                               v4 = iconst.i32 0
;; @0021                               v9 = iconst.i32 32
;; @0021                               v10 = iconst.i32 8
;; @0021                               v11 = call fn0(v0, v7, v4, v9, v10)  ; v7 = -1476395008, v4 = 0, v9 = 32, v10 = 8
;; @0021                               v15 = uextend.i64 v11
;; @0021                               v16 = iconst.i64 16
;; @0021                               v17 = uadd_overflow_trap v15, v16, user65535  ; v16 = 16
;; @0021                               v18 = iconst.i64 4
;; @0021                               v19 = uadd_overflow_trap v17, v18, user65535  ; v18 = 4
;; @0021                               v14 = load.i64 notrap aligned readonly v0+48
;; @0021                               v20 = icmp ult v19, v14
;; @0021                               brif v20, block5, block4
;;
;;                                 block4 cold:
;; @0021                               trap user65535
;;
;;                                 block5:
;; @0021                               v3 = f32const 0.0
;; @0021                               v13 = load.i64 notrap aligned readonly v0+40
;; @0021                               v21 = iadd v13, v17
;; @0021                               store notrap aligned v3, v21  ; v3 = 0.0
;; @0021                               v26 = iconst.i64 20
;; @0021                               v27 = uadd_overflow_trap.i64 v15, v26, user65535  ; v26 = 20
;; @0021                               v28 = iconst.i64 1
;; @0021                               v29 = uadd_overflow_trap v27, v28, user65535  ; v28 = 1
;; @0021                               v30 = icmp ult v29, v14
;; @0021                               brif v30, block7, block6
;;
;;                                 block6 cold:
;; @0021                               trap user65535
;;
;;                                 block7:
;;                                     v83 = iconst.i32 0
;; @0021                               v31 = iadd.i64 v13, v27
;; @0021                               istore8 notrap aligned v83, v31  ; v83 = 0
;; @0021                               v36 = iconst.i64 24
;; @0021                               v37 = uadd_overflow_trap.i64 v15, v36, user65535  ; v36 = 24
;;                                     v84 = iconst.i64 4
;; @0021                               v39 = uadd_overflow_trap v37, v84, user65535  ; v84 = 4
;; @0021                               v40 = icmp ult v39, v14
;; @0021                               brif v40, block9, block8
;;
;;                                 block8 cold:
;; @0021                               trap user65535
;;
;;                                 block9:
;;                                     v75 = iconst.i8 1
;; @0021                               brif v75, block3, block2  ; v75 = 1
;;
;;                                 block2:
;;                                     v82 = iconst.i64 0
;; @0021                               v49 = iconst.i64 8
;; @0021                               v50 = uadd_overflow_trap v82, v49, user65535  ; v82 = 0, v49 = 8
;; @0021                               v52 = uadd_overflow_trap v50, v49, user65535  ; v49 = 8
;; @0021                               v53 = icmp ult v52, v14
;; @0021                               brif v53, block11, block10
;;
;;                                 block10 cold:
;; @0021                               trap user65535
;;
;;                                 block11:
;; @0021                               v54 = iadd.i64 v13, v50
;; @0021                               v55 = load.i64 notrap aligned v54
;; @0021                               brif.i8 v53, block13, block12
;;
;;                                 block12 cold:
;; @0021                               trap user65535
;;
;;                                 block13:
;;                                     v85 = iconst.i64 1
;;                                     v86 = iadd.i64 v55, v85  ; v85 = 1
;; @0021                               store notrap aligned v86, v54
;; @0021                               jump block3
;;
;;                                 block3:
;;                                     v87 = iconst.i32 0
;; @0021                               v41 = iadd.i64 v13, v37
;; @0021                               store notrap aligned v87, v41  ; v87 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v11
;; }
