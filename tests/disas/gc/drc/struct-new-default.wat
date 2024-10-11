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
;; @0021                               v3 = f32const 0.0
;; @0021                               v13 = load.i64 notrap aligned readonly v0+40
;; @0021                               v14 = uextend.i64 v11
;; @0021                               v15 = iadd v13, v14
;;                                     v46 = iconst.i64 16
;; @0021                               v16 = iadd v15, v46  ; v46 = 16
;; @0021                               store notrap aligned little v3, v16  ; v3 = 0.0
;;                                     v47 = iconst.i64 20
;; @0021                               v17 = iadd v15, v47  ; v47 = 20
;; @0021                               istore8 notrap aligned little v4, v17  ; v4 = 0
;;                                     v57 = iconst.i8 1
;; @0021                               brif v57, block3, block2  ; v57 = 1
;;
;;                                 block2:
;;                                     v64 = iconst.i64 0
;; @0021                               v27 = iconst.i64 8
;; @0021                               v28 = uadd_overflow_trap v64, v27, user1  ; v64 = 0, v27 = 8
;; @0021                               v30 = uadd_overflow_trap v28, v27, user1  ; v27 = 8
;; @0021                               v25 = load.i64 notrap aligned readonly v0+48
;; @0021                               v31 = icmp ule v30, v25
;; @0021                               trapz v31, user1
;; @0021                               v32 = iadd.i64 v13, v28
;; @0021                               v33 = load.i64 notrap aligned v32
;; @0021                               trapz v31, user1
;;                                     v50 = iconst.i64 1
;; @0021                               v34 = iadd v33, v50  ; v50 = 1
;; @0021                               store notrap aligned v34, v32
;; @0021                               jump block3
;;
;;                                 block3:
;;                                     v65 = iconst.i32 0
;;                                     v48 = iconst.i64 24
;; @0021                               v18 = iadd.i64 v15, v48  ; v48 = 24
;; @0021                               store notrap aligned little v65, v18  ; v65 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v11
;; }
