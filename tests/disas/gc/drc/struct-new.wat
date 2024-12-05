;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i32 uext, i32 uext) -> i32 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v50 = stack_addr.i64 ss0
;;                                     store notrap v4, v50
;; @002a                               v8 = iconst.i32 -1342177280
;; @002a                               v9 = iconst.i32 0
;; @002a                               v6 = iconst.i32 32
;; @002a                               v10 = iconst.i32 8
;; @002a                               v11 = call fn0(v0, v8, v9, v6, v10), stack_map=[i32 @ ss0+0]  ; v8 = -1342177280, v9 = 0, v6 = 32, v10 = 8
;; @002a                               v13 = load.i64 notrap aligned readonly v0+40
;; @002a                               v14 = uextend.i64 v11
;; @002a                               v15 = iadd v13, v14
;;                                     v51 = iconst.i64 16
;; @002a                               v16 = iadd v15, v51  ; v51 = 16
;; @002a                               store notrap aligned little v2, v16
;;                                     v52 = iconst.i64 20
;; @002a                               v17 = iadd v15, v52  ; v52 = 20
;; @002a                               istore8 notrap aligned little v3, v17
;;                                     v49 = load.i32 notrap v50
;; @002a                               v19 = iconst.i32 -2
;; @002a                               v20 = band v49, v19  ; v19 = -2
;; @002a                               v21 = icmp eq v20, v9  ; v9 = 0
;; @002a                               brif v21, block3, block2
;;
;;                                 block2:
;; @002a                               v26 = uextend.i64 v49
;; @002a                               v27 = iconst.i64 8
;; @002a                               v28 = uadd_overflow_trap v26, v27, user1  ; v27 = 8
;; @002a                               v30 = uadd_overflow_trap v28, v27, user1  ; v27 = 8
;; @002a                               v25 = load.i64 notrap aligned readonly v0+48
;; @002a                               v31 = icmp ule v30, v25
;; @002a                               trapz v31, user1
;; @002a                               v32 = iadd.i64 v13, v28
;; @002a                               v33 = load.i64 notrap aligned v32
;;                                     v47 = load.i32 notrap v50
;; @002a                               v39 = uextend.i64 v47
;; @002a                               v41 = uadd_overflow_trap v39, v27, user1  ; v27 = 8
;; @002a                               v43 = uadd_overflow_trap v41, v27, user1  ; v27 = 8
;; @002a                               v44 = icmp ule v43, v25
;; @002a                               trapz v44, user1
;;                                     v57 = iconst.i64 1
;; @002a                               v34 = iadd v33, v57  ; v57 = 1
;; @002a                               v45 = iadd.i64 v13, v41
;; @002a                               store notrap aligned v34, v45
;; @002a                               jump block3
;;
;;                                 block3:
;;                                     v46 = load.i32 notrap v50
;;                                     v53 = iconst.i64 24
;; @002a                               v18 = iadd.i64 v15, v53  ; v53 = 24
;; @002a                               store notrap aligned little v46, v18
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v11
;; }
