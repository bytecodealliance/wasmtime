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
;; @002a                               v8 = iconst.i32 -1476395008
;; @002a                               v9 = iconst.i32 0
;; @002a                               v6 = iconst.i32 32
;; @002a                               v10 = iconst.i32 8
;; @002a                               v11 = call fn0(v0, v8, v9, v6, v10), stack_map=[i32 @ ss0+0]  ; v8 = -1476395008, v9 = 0, v6 = 32, v10 = 8
;; @002a                               v15 = uextend.i64 v11
;; @002a                               v16 = iconst.i64 16
;; @002a                               v17 = uadd_overflow_trap v15, v16, user65535  ; v16 = 16
;;                                     v78 = iconst.i64 32
;; @002a                               v19 = uadd_overflow_trap v15, v78, user65535  ; v78 = 32
;; @002a                               v14 = load.i64 notrap aligned readonly v0+48
;; @002a                               v20 = icmp ule v19, v14
;; @002a                               trapz v20, user65535
;; @002a                               v13 = load.i64 notrap aligned readonly v0+40
;; @002a                               v21 = iadd v13, v17
;; @002a                               store notrap aligned little v2, v21
;; @002a                               v26 = iconst.i64 20
;; @002a                               v27 = uadd_overflow_trap v15, v26, user65535  ; v26 = 20
;; @002a                               trapz v20, user65535
;; @002a                               v31 = iadd v13, v27
;; @002a                               istore8 notrap aligned little v3, v31
;; @002a                               v36 = iconst.i64 24
;; @002a                               v37 = uadd_overflow_trap v15, v36, user65535  ; v36 = 24
;; @002a                               trapz v20, user65535
;;                                     v70 = load.i32 notrap v71
;; @002a                               v42 = iconst.i32 -2
;; @002a                               v43 = band v70, v42  ; v42 = -2
;; @002a                               v44 = icmp eq v43, v9  ; v9 = 0
;; @002a                               brif v44, block3, block2
;;
;;                                 block2:
;; @002a                               v48 = uextend.i64 v70
;; @002a                               v49 = iconst.i64 8
;; @002a                               v50 = uadd_overflow_trap v48, v49, user65535  ; v49 = 8
;; @002a                               v52 = uadd_overflow_trap v50, v49, user65535  ; v49 = 8
;; @002a                               v53 = icmp ule v52, v14
;; @002a                               trapz v53, user65535
;; @002a                               v54 = iadd.i64 v13, v50
;; @002a                               v55 = load.i64 notrap aligned v54
;;                                     v68 = load.i32 notrap v71
;; @002a                               v60 = uextend.i64 v68
;; @002a                               v62 = uadd_overflow_trap v60, v49, user65535  ; v49 = 8
;; @002a                               v64 = uadd_overflow_trap v62, v49, user65535  ; v49 = 8
;; @002a                               v65 = icmp ule v64, v14
;; @002a                               trapz v65, user65535
;;                                     v75 = iconst.i64 1
;; @002a                               v56 = iadd v55, v75  ; v75 = 1
;; @002a                               v66 = iadd.i64 v13, v62
;; @002a                               store notrap aligned v56, v66
;; @002a                               jump block3
;;
;;                                 block3:
;;                                     v67 = load.i32 notrap v71
;; @002a                               v41 = iadd.i64 v13, v37
;; @002a                               store notrap aligned little v67, v41
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v11
;; }
