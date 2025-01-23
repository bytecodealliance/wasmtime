;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
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
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v4 = iconst.i32 0
;; @0021                               trapnz v4, user18  ; v4 = 0
;; @0021                               v11 = load.i64 notrap aligned readonly v0+56
;; @0021                               v12 = load.i32 notrap aligned v11
;;                                     v44 = iconst.i32 7
;; @0021                               v15 = uadd_overflow_trap v12, v44, user18  ; v44 = 7
;;                                     v51 = iconst.i32 -8
;; @0021                               v17 = band v15, v51  ; v51 = -8
;; @0021                               v6 = iconst.i32 24
;; @0021                               v18 = uadd_overflow_trap v17, v6, user18  ; v6 = 24
;; @0021                               v19 = uextend.i64 v18
;; @0021                               v23 = load.i64 notrap aligned readonly v0+48
;; @0021                               v24 = icmp ule v19, v23
;; @0021                               trapz v24, user18
;;                                     v52 = iconst.i32 -1342177256
;; @0021                               v21 = load.i64 notrap aligned readonly v0+40
;; @0021                               v25 = uextend.i64 v17
;; @0021                               v26 = iadd v21, v25
;; @0021                               store notrap aligned v52, v26  ; v52 = -1342177256
;; @0021                               v30 = load.i64 notrap aligned readonly v0+64
;; @0021                               v31 = load.i32 notrap aligned readonly v30
;; @0021                               store notrap aligned v31, v26+4
;; @0021                               store notrap aligned v18, v11
;; @0021                               v3 = f32const 0.0
;;                                     v35 = iconst.i64 8
;; @0021                               v32 = iadd v26, v35  ; v35 = 8
;; @0021                               store notrap aligned little v3, v32  ; v3 = 0.0
;;                                     v36 = iconst.i64 12
;; @0021                               v33 = iadd v26, v36  ; v36 = 12
;; @0021                               istore8 notrap aligned little v4, v33  ; v4 = 0
;;                                     v37 = iconst.i64 16
;; @0021                               v34 = iadd v26, v37  ; v37 = 16
;; @0021                               store notrap aligned little v4, v34  ; v4 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;;                                     v59 = band.i32 v15, v51  ; v51 = -8
;; @0024                               return v59
;; }
