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
;;     gv2 = load.i64 notrap aligned gv1
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i32 uext, i32 uext, i32 uext, i32 uext) -> i32 system_v
;;     fn0 = colocated u1:27 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v8 = iconst.i32 -1342177280
;; @0021                               v4 = iconst.i32 0
;; @0021                               v6 = iconst.i32 32
;; @0021                               v10 = iconst.i32 8
;; @0021                               v11 = call fn0(v0, v8, v4, v6, v10)  ; v8 = -1342177280, v4 = 0, v6 = 32, v10 = 8
;; @0021                               v16 = uextend.i64 v11
;; @0021                               v17 = iconst.i64 16
;; @0021                               v18 = uadd_overflow_trap v16, v17, user1  ; v17 = 16
;;                                     v74 = iconst.i64 32
;; @0021                               v20 = uadd_overflow_trap v16, v74, user1  ; v74 = 32
;; @0021                               v15 = load.i64 notrap aligned readonly v0+48
;; @0021                               v21 = icmp ule v20, v15
;; @0021                               trapz v21, user1
;; @0021                               v3 = f32const 0.0
;; @0021                               v13 = load.i64 notrap aligned readonly v0+40
;; @0021                               v22 = iadd v13, v18
;; @0021                               store notrap aligned little v3, v22  ; v3 = 0.0
;; @0021                               v28 = iconst.i64 20
;; @0021                               v29 = uadd_overflow_trap v16, v28, user1  ; v28 = 20
;; @0021                               trapz v21, user1
;; @0021                               v33 = iadd v13, v29
;; @0021                               istore8 notrap aligned little v4, v33  ; v4 = 0
;; @0021                               v39 = iconst.i64 24
;; @0021                               v40 = uadd_overflow_trap v16, v39, user1  ; v39 = 24
;; @0021                               trapz v21, user1
;;                                     v81 = iconst.i8 1
;; @0021                               brif v81, block3, block2  ; v81 = 1
;;
;;                                 block2:
;;                                     v88 = iconst.i64 0
;; @0021                               v53 = iconst.i64 8
;; @0021                               v54 = uadd_overflow_trap v88, v53, user1  ; v88 = 0, v53 = 8
;; @0021                               v56 = uadd_overflow_trap v54, v53, user1  ; v53 = 8
;; @0021                               v57 = icmp ule v56, v15
;; @0021                               trapz v57, user1
;; @0021                               v58 = iadd.i64 v13, v54
;; @0021                               v59 = load.i64 notrap aligned v58
;; @0021                               trapz v57, user1
;;                                     v73 = iconst.i64 1
;; @0021                               v60 = iadd v59, v73  ; v73 = 1
;; @0021                               store notrap aligned v60, v58
;; @0021                               jump block3
;;
;;                                 block3:
;;                                     v89 = iconst.i32 0
;; @0021                               v44 = iadd.i64 v13, v40
;; @0021                               store notrap aligned little v89, v44  ; v89 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v11
;; }
