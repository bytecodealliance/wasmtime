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
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i32 uext, i32 uext) -> i64 tail
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v8 = iconst.i32 -1342177280
;; @0021                               v4 = iconst.i32 0
;; @0021                               v6 = iconst.i32 32
;; @0021                               v10 = iconst.i32 8
;; @0021                               v11 = call fn0(v0, v8, v4, v6, v10)  ; v8 = -1342177280, v4 = 0, v6 = 32, v10 = 8
;; @0021                               v3 = f32const 0.0
;; @0021                               v14 = load.i64 notrap aligned readonly v0+40
;; @0021                               v12 = ireduce.i32 v11
;; @0021                               v15 = uextend.i64 v12
;; @0021                               v16 = iadd v14, v15
;;                                     v47 = iconst.i64 16
;; @0021                               v17 = iadd v16, v47  ; v47 = 16
;; @0021                               store notrap aligned little v3, v17  ; v3 = 0.0
;;                                     v48 = iconst.i64 20
;; @0021                               v18 = iadd v16, v48  ; v48 = 20
;; @0021                               istore8 notrap aligned little v4, v18  ; v4 = 0
;;                                     v58 = iconst.i8 1
;; @0021                               brif v58, block3, block2  ; v58 = 1
;;
;;                                 block2:
;;                                     v65 = iconst.i64 0
;; @0021                               v28 = iconst.i64 8
;; @0021                               v29 = uadd_overflow_trap v65, v28, user1  ; v65 = 0, v28 = 8
;; @0021                               v31 = uadd_overflow_trap v29, v28, user1  ; v28 = 8
;; @0021                               v26 = load.i64 notrap aligned readonly v0+48
;; @0021                               v32 = icmp ule v31, v26
;; @0021                               trapz v32, user1
;; @0021                               v33 = iadd.i64 v14, v29
;; @0021                               v34 = load.i64 notrap aligned v33
;; @0021                               trapz v32, user1
;;                                     v51 = iconst.i64 1
;; @0021                               v35 = iadd v34, v51  ; v51 = 1
;; @0021                               store notrap aligned v35, v33
;; @0021                               jump block3
;;
;;                                 block3:
;;                                     v66 = iconst.i32 0
;;                                     v49 = iconst.i64 24
;; @0021                               v19 = iadd.i64 v16, v49  ; v49 = 24
;; @0021                               store notrap aligned little v66, v19  ; v66 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v12
;; }
