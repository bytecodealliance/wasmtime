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
;; @0021                               v8 = iconst.i32 -1476395008
;; @0021                               v4 = iconst.i32 0
;; @0021                               v6 = iconst.i32 32
;; @0021                               v10 = iconst.i32 8
;; @0021                               v11 = call fn0(v0, v8, v4, v6, v10)  ; v8 = -1476395008, v4 = 0, v6 = 32, v10 = 8
;; @0021                               v15 = uextend.i64 v11
;; @0021                               v16 = iconst.i64 16
;; @0021                               v17 = uadd_overflow_trap v15, v16, user1  ; v16 = 16
;;                                     v69 = iconst.i64 32
;; @0021                               v19 = uadd_overflow_trap v15, v69, user1  ; v69 = 32
;; @0021                               v14 = load.i64 notrap aligned readonly v0+48
;; @0021                               v20 = icmp ule v19, v14
;; @0021                               trapz v20, user1
;; @0021                               v3 = f32const 0.0
;; @0021                               v13 = load.i64 notrap aligned readonly v0+40
;; @0021                               v21 = iadd v13, v17
;; @0021                               store notrap aligned little v3, v21  ; v3 = 0.0
;; @0021                               v26 = iconst.i64 20
;; @0021                               v27 = uadd_overflow_trap v15, v26, user1  ; v26 = 20
;; @0021                               trapz v20, user1
;; @0021                               v31 = iadd v13, v27
;; @0021                               istore8 notrap aligned little v4, v31  ; v4 = 0
;; @0021                               v36 = iconst.i64 24
;; @0021                               v37 = uadd_overflow_trap v15, v36, user1  ; v36 = 24
;; @0021                               trapz v20, user1
;;                                     v76 = iconst.i8 1
;; @0021                               brif v76, block3, block2  ; v76 = 1
;;
;;                                 block2:
;;                                     v83 = iconst.i64 0
;; @0021                               v49 = iconst.i64 8
;; @0021                               v50 = uadd_overflow_trap v83, v49, user1  ; v83 = 0, v49 = 8
;; @0021                               v52 = uadd_overflow_trap v50, v49, user1  ; v49 = 8
;; @0021                               v53 = icmp ule v52, v14
;; @0021                               trapz v53, user1
;; @0021                               v54 = iadd.i64 v13, v50
;; @0021                               v55 = load.i64 notrap aligned v54
;; @0021                               trapz v53, user1
;;                                     v68 = iconst.i64 1
;; @0021                               v56 = iadd v55, v68  ; v68 = 1
;; @0021                               store notrap aligned v56, v54
;; @0021                               jump block3
;;
;;                                 block3:
;;                                     v84 = iconst.i32 0
;; @0021                               v41 = iadd.i64 v13, v37
;; @0021                               store notrap aligned little v84, v41  ; v84 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v11
;; }
