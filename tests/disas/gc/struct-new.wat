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
;;                                     v76 = stack_addr.i64 ss0
;;                                     store notrap v4, v76
;; @002a                               v8 = iconst.i32 -1342177280
;; @002a                               v9 = iconst.i32 0
;; @002a                               v6 = iconst.i32 32
;; @002a                               v10 = iconst.i32 8
;; @002a                               v11 = call fn0(v0, v8, v9, v6, v10), stack_map=[i32 @ ss0+0]  ; v8 = -1342177280, v9 = 0, v6 = 32, v10 = 8
;; @002a                               v16 = uextend.i64 v11
;; @002a                               v17 = iconst.i64 16
;; @002a                               v18 = uadd_overflow_trap v16, v17, user1  ; v17 = 16
;;                                     v83 = iconst.i64 32
;; @002a                               v20 = uadd_overflow_trap v16, v83, user1  ; v83 = 32
;; @002a                               v15 = load.i64 notrap aligned readonly v0+48
;; @002a                               v21 = icmp ule v20, v15
;; @002a                               trapz v21, user1
;; @002a                               v13 = load.i64 notrap aligned readonly v0+40
;; @002a                               v22 = iadd v13, v18
;; @002a                               store notrap aligned little v2, v22
;; @002a                               v28 = iconst.i64 20
;; @002a                               v29 = uadd_overflow_trap v16, v28, user1  ; v28 = 20
;; @002a                               trapz v21, user1
;; @002a                               v33 = iadd v13, v29
;; @002a                               istore8 notrap aligned little v3, v33
;; @002a                               v39 = iconst.i64 24
;; @002a                               v40 = uadd_overflow_trap v16, v39, user1  ; v39 = 24
;; @002a                               trapz v21, user1
;;                                     v75 = load.i32 notrap v76
;; @002a                               v45 = iconst.i32 -2
;; @002a                               v46 = band v75, v45  ; v45 = -2
;; @002a                               v47 = icmp eq v46, v9  ; v9 = 0
;; @002a                               brif v47, block3, block2
;;
;;                                 block2:
;; @002a                               v52 = uextend.i64 v75
;; @002a                               v53 = iconst.i64 8
;; @002a                               v54 = uadd_overflow_trap v52, v53, user1  ; v53 = 8
;; @002a                               v56 = uadd_overflow_trap v54, v53, user1  ; v53 = 8
;; @002a                               v57 = icmp ule v56, v15
;; @002a                               trapz v57, user1
;; @002a                               v58 = iadd.i64 v13, v54
;; @002a                               v59 = load.i64 notrap aligned v58
;;                                     v73 = load.i32 notrap v76
;; @002a                               v65 = uextend.i64 v73
;; @002a                               v67 = uadd_overflow_trap v65, v53, user1  ; v53 = 8
;; @002a                               v69 = uadd_overflow_trap v67, v53, user1  ; v53 = 8
;; @002a                               v70 = icmp ule v69, v15
;; @002a                               trapz v70, user1
;;                                     v80 = iconst.i64 1
;; @002a                               v60 = iadd v59, v80  ; v80 = 1
;; @002a                               v71 = iadd.i64 v13, v67
;; @002a                               store notrap aligned v60, v71
;; @002a                               jump block3
;;
;;                                 block3:
;;                                     v72 = load.i32 notrap v76
;; @002a                               v44 = iadd.i64 v13, v40
;; @002a                               store notrap aligned little v72, v44
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v11
;; }
