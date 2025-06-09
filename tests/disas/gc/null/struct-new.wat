;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned gv4+32
;;     gv6 = load.i64 notrap aligned readonly can_move gv4+24
;;     sig0 = (i64 vmctx, i64) -> i8 tail
;;     fn0 = colocated u1:26 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v45 = stack_addr.i64 ss0
;;                                     store notrap v4, v45
;; @002a                               v11 = load.i64 notrap aligned readonly v0+40
;; @002a                               v12 = load.i32 notrap aligned v11
;;                                     v53 = iconst.i32 7
;; @002a                               v15 = uadd_overflow_trap v12, v53, user18  ; v53 = 7
;;                                     v60 = iconst.i32 -8
;; @002a                               v17 = band v15, v60  ; v60 = -8
;; @002a                               v6 = iconst.i32 24
;; @002a                               v18 = uadd_overflow_trap v17, v6, user18  ; v6 = 24
;; @002a                               v43 = load.i64 notrap aligned readonly can_move v0+8
;; @002a                               v20 = load.i64 notrap aligned v43+32
;; @002a                               v19 = uextend.i64 v18
;; @002a                               v21 = icmp ule v19, v20
;; @002a                               brif v21, block2, block3
;;
;;                                 block2:
;;                                     v61 = iconst.i32 -1342177256
;; @002a                               v25 = load.i64 notrap aligned readonly can_move v43+24
;;                                     v68 = band.i32 v15, v60  ; v60 = -8
;;                                     v69 = uextend.i64 v68
;; @002a                               v27 = iadd v25, v69
;; @002a                               store notrap aligned v61, v27  ; v61 = -1342177256
;; @002a                               v31 = load.i64 notrap aligned readonly can_move v0+48
;; @002a                               v32 = load.i32 notrap aligned readonly can_move v31
;; @002a                               store notrap aligned v32, v27+4
;; @002a                               store.i32 notrap aligned v18, v11
;;                                     v40 = iconst.i64 8
;; @002a                               v33 = iadd v27, v40  ; v40 = 8
;; @002a                               store.f32 notrap aligned little v2, v33
;;                                     v39 = iconst.i64 12
;; @002a                               v34 = iadd v27, v39  ; v39 = 12
;; @002a                               istore8.i32 notrap aligned little v3, v34
;;                                     v36 = load.i32 notrap v45
;;                                     v38 = iconst.i64 16
;; @002a                               v35 = iadd v27, v38  ; v38 = 16
;; @002a                               store notrap aligned little v36, v35
;; @002d                               jump block1
;;
;;                                 block3 cold:
;; @002a                               v23 = isub.i64 v19, v20
;; @002a                               v24 = call fn0(v0, v23), stack_map=[i32 @ ss0+0]
;; @002a                               jump block2
;;
;;                                 block1:
;;                                     v70 = band.i32 v15, v60  ; v60 = -8
;; @002d                               return v70
;; }
