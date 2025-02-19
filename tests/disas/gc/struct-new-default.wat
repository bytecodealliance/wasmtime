;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))
                    (field (mut v128))))

  (func (result (ref $ty))
    (struct.new_default $ty)
  )
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0023                               v9 = iconst.i32 -1342177280
;; @0023                               v4 = iconst.i32 0
;; @0023                               v7 = iconst.i32 48
;; @0023                               v11 = iconst.i32 16
;; @0023                               v12 = call fn0(v0, v9, v4, v7, v11)  ; v9 = -1342177280, v4 = 0, v7 = 48, v11 = 16
;; @0023                               v3 = f32const 0.0
;; @0023                               v15 = load.i64 notrap aligned readonly v0+40
;; @0023                               v13 = ireduce.i32 v12
;; @0023                               v16 = uextend.i64 v13
;; @0023                               v17 = iadd v15, v16
;;                                     v50 = iconst.i64 16
;; @0023                               v18 = iadd v17, v50  ; v50 = 16
;; @0023                               store notrap aligned little v3, v18  ; v3 = 0.0
;;                                     v51 = iconst.i64 20
;; @0023                               v19 = iadd v17, v51  ; v51 = 20
;; @0023                               istore8 notrap aligned little v4, v19  ; v4 = 0
;;                                     v53 = iconst.i32 1
;; @0023                               brif v53, block3, block2  ; v53 = 1
;;
;;                                 block2:
;;                                     v76 = iconst.i64 0
;; @0023                               v30 = iconst.i64 8
;; @0023                               v31 = uadd_overflow_trap v76, v30, user1  ; v76 = 0, v30 = 8
;; @0023                               v33 = uadd_overflow_trap v31, v30, user1  ; v30 = 8
;; @0023                               v28 = load.i64 notrap aligned readonly v0+48
;; @0023                               v34 = icmp ule v33, v28
;; @0023                               trapz v34, user1
;; @0023                               v35 = iadd.i64 v15, v31
;; @0023                               v36 = load.i64 notrap aligned v35
;;                                     v55 = iconst.i64 1
;; @0023                               v37 = iadd v36, v55  ; v55 = 1
;; @0023                               store notrap aligned v37, v35
;; @0023                               jump block3
;;
;;                                 block3:
;;                                     v77 = iconst.i32 0
;;                                     v52 = iconst.i64 24
;; @0023                               v20 = iadd.i64 v17, v52  ; v52 = 24
;; @0023                               store notrap aligned little v77, v20  ; v77 = 0
;; @0023                               v6 = vconst.i8x16 const0
;;                                     v56 = iconst.i64 32
;; @0023                               v49 = iadd.i64 v17, v56  ; v56 = 32
;; @0023                               store notrap aligned little v6, v49  ; v6 = const0
;; @0026                               jump block1
;;
;;                                 block1:
;; @0026                               return v13
;; }
