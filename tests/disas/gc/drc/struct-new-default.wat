;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;;                                     v55 = iconst.i32 -1342177279
;; @0021                               v4 = iconst.i32 0
;; @0021                               v6 = iconst.i32 40
;; @0021                               v12 = iconst.i32 8
;; @0021                               v13 = call fn0(v0, v55, v4, v6, v12)  ; v55 = -1342177279, v4 = 0, v6 = 40, v12 = 8
;; @0021                               v3 = f32const 0.0
;; @0021                               v15 = load.i64 notrap aligned readonly can_move v0+40
;; @0021                               v16 = uextend.i64 v13
;; @0021                               v17 = iadd v15, v16
;;                                     v49 = iconst.i64 28
;; @0021                               v18 = iadd v17, v49  ; v49 = 28
;; @0021                               store notrap aligned little v3, v18  ; v3 = 0.0
;;                                     v50 = iconst.i64 32
;; @0021                               v19 = iadd v17, v50  ; v50 = 32
;; @0021                               istore8 notrap aligned little v4, v19  ; v4 = 0
;; @0021                               v7 = iconst.i32 1
;; @0021                               brif v7, block3, block2  ; v7 = 1
;;
;;                                 block2:
;;                                     v81 = iconst.i64 0
;; @0021                               v30 = iconst.i64 8
;; @0021                               v31 = uadd_overflow_trap v81, v30, user1  ; v81 = 0, v30 = 8
;; @0021                               v33 = uadd_overflow_trap v31, v30, user1  ; v30 = 8
;; @0021                               v28 = load.i64 notrap aligned readonly can_move v0+48
;; @0021                               v34 = icmp ule v33, v28
;; @0021                               trapz v34, user1
;; @0021                               v35 = iadd.i64 v15, v31
;; @0021                               v36 = load.i64 notrap aligned v35
;;                                     v54 = iconst.i64 1
;; @0021                               v37 = iadd v36, v54  ; v54 = 1
;; @0021                               store notrap aligned v37, v35
;; @0021                               jump block3
;;
;;                                 block3:
;;                                     v82 = iconst.i32 0
;;                                     v51 = iconst.i64 24
;; @0021                               v20 = iadd.i64 v17, v51  ; v51 = 24
;; @0021                               store notrap aligned little v82, v20  ; v82 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v13
;; }
