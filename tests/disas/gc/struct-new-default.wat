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
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i32 uext, i32 uext) -> i64 tail
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
;;                                     v49 = iconst.i64 16
;; @0023                               v18 = iadd v17, v49  ; v49 = 16
;; @0023                               store notrap aligned little v3, v18  ; v3 = 0.0
;;                                     v50 = iconst.i64 20
;; @0023                               v19 = iadd v17, v50  ; v50 = 20
;; @0023                               istore8 notrap aligned little v4, v19  ; v4 = 0
;;                                     v61 = iconst.i8 1
;; @0023                               brif v61, block3, block2  ; v61 = 1
;;
;;                                 block2:
;;                                     v68 = iconst.i64 0
;; @0023                               v29 = iconst.i64 8
;; @0023                               v30 = uadd_overflow_trap v68, v29, user1  ; v68 = 0, v29 = 8
;; @0023                               v32 = uadd_overflow_trap v30, v29, user1  ; v29 = 8
;; @0023                               v27 = load.i64 notrap aligned readonly v0+48
;; @0023                               v33 = icmp ule v32, v27
;; @0023                               trapz v33, user1
;; @0023                               v34 = iadd.i64 v15, v30
;; @0023                               v35 = load.i64 notrap aligned v34
;; @0023                               trapz v33, user1
;;                                     v53 = iconst.i64 1
;; @0023                               v36 = iadd v35, v53  ; v53 = 1
;; @0023                               store notrap aligned v36, v34
;; @0023                               jump block3
;;
;;                                 block3:
;;                                     v69 = iconst.i32 0
;;                                     v51 = iconst.i64 24
;; @0023                               v20 = iadd.i64 v17, v51  ; v51 = 24
;; @0023                               store notrap aligned little v69, v20  ; v69 = 0
;; @0023                               v6 = vconst.i8x16 const0
;;                                     v54 = iconst.i64 32
;; @0023                               v48 = iadd.i64 v17, v54  ; v54 = 32
;; @0023                               store notrap aligned little v6, v48  ; v6 = const0
;; @0026                               jump block1
;;
;;                                 block1:
;; @0026                               return v13
;; }
