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
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v54 = stack_addr.i64 ss0
;;                                     store notrap v4, v54
;;                                     v66 = iconst.i32 -1342177279
;; @002a                               v11 = iconst.i32 0
;; @002a                               v6 = iconst.i32 40
;; @002a                               v12 = iconst.i32 8
;; @002a                               v13 = call fn0(v0, v66, v11, v6, v12), stack_map=[i32 @ ss0+0]  ; v66 = -1342177279, v11 = 0, v6 = 40, v12 = 8
;; @002a                               v15 = load.i64 notrap aligned readonly can_move v0+40
;; @002a                               v16 = uextend.i64 v13
;; @002a                               v17 = iadd v15, v16
;;                                     v55 = iconst.i64 28
;; @002a                               v18 = iadd v17, v55  ; v55 = 28
;; @002a                               store notrap aligned little v2, v18
;;                                     v56 = iconst.i64 32
;; @002a                               v19 = iadd v17, v56  ; v56 = 32
;; @002a                               istore8 notrap aligned little v3, v19
;;                                     v53 = load.i32 notrap v54
;; @002a                               v7 = iconst.i32 1
;; @002a                               v21 = band v53, v7  ; v7 = 1
;; @002a                               v22 = icmp eq v53, v11  ; v11 = 0
;; @002a                               v23 = uextend.i32 v22
;; @002a                               v24 = bor v21, v23
;; @002a                               brif v24, block3, block2
;;
;;                                 block2:
;; @002a                               v29 = uextend.i64 v53
;; @002a                               v30 = iconst.i64 8
;; @002a                               v31 = uadd_overflow_trap v29, v30, user1  ; v30 = 8
;; @002a                               v33 = uadd_overflow_trap v31, v30, user1  ; v30 = 8
;; @002a                               v28 = load.i64 notrap aligned readonly can_move v0+48
;; @002a                               v34 = icmp ule v33, v28
;; @002a                               trapz v34, user1
;; @002a                               v35 = iadd.i64 v15, v31
;; @002a                               v36 = load.i64 notrap aligned v35
;;                                     v63 = iconst.i64 1
;; @002a                               v37 = iadd v36, v63  ; v63 = 1
;; @002a                               store notrap aligned v37, v35
;; @002a                               jump block3
;;
;;                                 block3:
;;                                     v49 = load.i32 notrap v54
;;                                     v57 = iconst.i64 24
;; @002a                               v20 = iadd.i64 v17, v57  ; v57 = 24
;; @002a                               store notrap aligned little v49, v20
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v13
;; }
