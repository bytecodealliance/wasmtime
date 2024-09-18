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
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i32 uext, i32 uext) -> i32 system_v
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v71 = stack_addr.i64 ss0
;;                                     store notrap v4, v71
;; @002a                               v7 = iconst.i32 -1476395008
;; @002a                               v8 = iconst.i32 0
;; @002a                               v9 = iconst.i32 32
;; @002a                               v10 = iconst.i32 8
;; @002a                               v11 = call fn0(v0, v7, v8, v9, v10), stack_map=[i32 @ ss0+0]  ; v7 = -1476395008, v8 = 0, v9 = 32, v10 = 8
;; @002a                               v15 = uextend.i64 v11
;; @002a                               v16 = iconst.i64 16
;; @002a                               v17 = uadd_overflow_trap v15, v16, user65535  ; v16 = 16
;; @002a                               v18 = iconst.i64 4
;; @002a                               v19 = uadd_overflow_trap v17, v18, user65535  ; v18 = 4
;; @002a                               v14 = load.i64 notrap aligned readonly v0+48
;; @002a                               v20 = icmp ult v19, v14
;; @002a                               brif v20, block5, block4
;;
;;                                 block4 cold:
;; @002a                               trap user65535
;;
;;                                 block5:
;; @002a                               v13 = load.i64 notrap aligned readonly v0+40
;; @002a                               v21 = iadd v13, v17
;; @002a                               store.f32 notrap aligned v2, v21
;; @002a                               v26 = iconst.i64 20
;; @002a                               v27 = uadd_overflow_trap.i64 v15, v26, user65535  ; v26 = 20
;; @002a                               v28 = iconst.i64 1
;; @002a                               v29 = uadd_overflow_trap v27, v28, user65535  ; v28 = 1
;; @002a                               v30 = icmp ult v29, v14
;; @002a                               brif v30, block7, block6
;;
;;                                 block6 cold:
;; @002a                               trap user65535
;;
;;                                 block7:
;; @002a                               v31 = iadd.i64 v13, v27
;; @002a                               istore8.i32 notrap aligned v3, v31
;; @002a                               v36 = iconst.i64 24
;; @002a                               v37 = uadd_overflow_trap.i64 v15, v36, user65535  ; v36 = 24
;;                                     v78 = iconst.i64 4
;; @002a                               v39 = uadd_overflow_trap v37, v78, user65535  ; v78 = 4
;; @002a                               v40 = icmp ult v39, v14
;; @002a                               brif v40, block9, block8
;;
;;                                 block8 cold:
;; @002a                               trap user65535
;;
;;                                 block9:
;;                                     v70 = load.i32 notrap v71
;; @002a                               v42 = iconst.i32 -2
;; @002a                               v43 = band v70, v42  ; v42 = -2
;;                                     v79 = iconst.i32 0
;;                                     v80 = icmp eq v43, v79  ; v79 = 0
;; @002a                               brif v80, block3, block2
;;
;;                                 block2:
;; @002a                               v48 = uextend.i64 v70
;; @002a                               v49 = iconst.i64 8
;; @002a                               v50 = uadd_overflow_trap v48, v49, user65535  ; v49 = 8
;; @002a                               v52 = uadd_overflow_trap v50, v49, user65535  ; v49 = 8
;; @002a                               v53 = icmp ult v52, v14
;; @002a                               brif v53, block11, block10
;;
;;                                 block10 cold:
;; @002a                               trap user65535
;;
;;                                 block11:
;; @002a                               v54 = iadd.i64 v13, v50
;; @002a                               v55 = load.i64 notrap aligned v54
;;                                     v68 = load.i32 notrap v71
;; @002a                               v60 = uextend.i64 v68
;;                                     v81 = iconst.i64 8
;; @002a                               v62 = uadd_overflow_trap v60, v81, user65535  ; v81 = 8
;; @002a                               v64 = uadd_overflow_trap v62, v81, user65535  ; v81 = 8
;; @002a                               v65 = icmp ult v64, v14
;; @002a                               brif v65, block13, block12
;;
;;                                 block12 cold:
;; @002a                               trap user65535
;;
;;                                 block13:
;;                                     v82 = iconst.i64 1
;;                                     v83 = iadd.i64 v55, v82  ; v82 = 1
;; @002a                               v66 = iadd.i64 v13, v62
;; @002a                               store notrap aligned v83, v66
;; @002a                               jump block3
;;
;;                                 block3:
;;                                     v67 = load.i32 notrap v71
;; @002a                               v41 = iadd.i64 v13, v37
;; @002a                               store notrap aligned v67, v41
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v11
;; }
