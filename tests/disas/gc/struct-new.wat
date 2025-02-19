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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v53 = stack_addr.i64 ss0
;;                                     store notrap v4, v53
;; @002a                               v8 = iconst.i32 -1342177280
;; @002a                               v9 = iconst.i32 0
;; @002a                               v6 = iconst.i32 32
;; @002a                               v10 = iconst.i32 8
;; @002a                               v11 = call fn0(v0, v8, v9, v6, v10), stack_map=[i32 @ ss0+0]  ; v8 = -1342177280, v9 = 0, v6 = 32, v10 = 8
;; @002a                               v14 = load.i64 notrap aligned readonly v0+40
;; @002a                               v12 = ireduce.i32 v11
;; @002a                               v15 = uextend.i64 v12
;; @002a                               v16 = iadd v14, v15
;;                                     v54 = iconst.i64 16
;; @002a                               v17 = iadd v16, v54  ; v54 = 16
;; @002a                               store notrap aligned little v2, v17
;;                                     v55 = iconst.i64 20
;; @002a                               v18 = iadd v16, v55  ; v55 = 20
;; @002a                               istore8 notrap aligned little v3, v18
;;                                     v52 = load.i32 notrap v53
;;                                     v58 = iconst.i32 1
;; @002a                               v20 = band v52, v58  ; v58 = 1
;; @002a                               v21 = icmp eq v52, v9  ; v9 = 0
;; @002a                               v22 = uextend.i32 v21
;; @002a                               v23 = bor v20, v22
;; @002a                               brif v23, block3, block2
;;
;;                                 block2:
;; @002a                               v28 = uextend.i64 v52
;; @002a                               v29 = iconst.i64 8
;; @002a                               v30 = uadd_overflow_trap v28, v29, user1  ; v29 = 8
;; @002a                               v32 = uadd_overflow_trap v30, v29, user1  ; v29 = 8
;; @002a                               v27 = load.i64 notrap aligned readonly v0+48
;; @002a                               v33 = icmp ule v32, v27
;; @002a                               trapz v33, user1
;; @002a                               v34 = iadd.i64 v14, v30
;; @002a                               v35 = load.i64 notrap aligned v34
;;                                     v62 = iconst.i64 1
;; @002a                               v36 = iadd v35, v62  ; v62 = 1
;; @002a                               store notrap aligned v36, v34
;; @002a                               jump block3
;;
;;                                 block3:
;;                                     v48 = load.i32 notrap v53
;;                                     v56 = iconst.i64 24
;; @002a                               v19 = iadd.i64 v16, v56  ; v56 = 24
;; @002a                               store notrap aligned little v48, v19
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v12
;; }
