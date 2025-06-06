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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+32
;;     gv6 = load.i64 notrap aligned readonly can_move gv4+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v11 = load.i64 notrap aligned readonly v0+40
;; @0021                               v12 = load.i32 notrap aligned v11
;;                                     v49 = iconst.i32 7
;; @0021                               v15 = uadd_overflow_trap v12, v49, user18  ; v49 = 7
;;                                     v56 = iconst.i32 -8
;; @0021                               v17 = band v15, v56  ; v56 = -8
;; @0021                               v6 = iconst.i32 24
;; @0021                               v18 = uadd_overflow_trap v17, v6, user18  ; v6 = 24
;; @0021                               v41 = load.i64 notrap aligned readonly can_move v0+8
;; @0021                               v20 = load.i64 notrap aligned v41+32
;; @0021                               v19 = uextend.i64 v18
;; @0021                               v21 = icmp ule v19, v20
;; @0021                               brif v21, block2, block3
;;
;;                                 block2:
;;                                     v57 = iconst.i32 -1342177256
;; @0021                               v25 = load.i64 notrap aligned readonly can_move v41+24
;;                                     v64 = band.i32 v15, v56  ; v56 = -8
;;                                     v65 = uextend.i64 v64
;; @0021                               v27 = iadd v25, v65
;; @0021                               store notrap aligned v57, v27  ; v57 = -1342177256
;; @0021                               v31 = load.i64 notrap aligned readonly can_move v0+48
;; @0021                               v32 = load.i32 notrap aligned readonly can_move v31
;; @0021                               store notrap aligned v32, v27+4
;; @0021                               store.i32 notrap aligned v18, v11
;; @0021                               v3 = f32const 0.0
;;                                     v38 = iconst.i64 8
;; @0021                               v33 = iadd v27, v38  ; v38 = 8
;; @0021                               store notrap aligned little v3, v33  ; v3 = 0.0
;; @0021                               v4 = iconst.i32 0
;;                                     v37 = iconst.i64 12
;; @0021                               v34 = iadd v27, v37  ; v37 = 12
;; @0021                               istore8 notrap aligned little v4, v34  ; v4 = 0
;;                                     v36 = iconst.i64 16
;; @0021                               v35 = iadd v27, v36  ; v36 = 16
;; @0021                               store notrap aligned little v4, v35  ; v4 = 0
;; @0024                               jump block1
;;
;;                                 block3 cold:
;; @0021                               v23 = isub.i64 v19, v20
;; @0021                               v24 = call fn0(v0, v23)
;; @0021                               jump block2
;;
;;                                 block1:
;;                                     v66 = band.i32 v15, v56  ; v56 = -8
;; @0024                               return v66
;; }
