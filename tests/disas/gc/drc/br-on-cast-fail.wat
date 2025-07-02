;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=drc"
;;! test = "optimize"

(module
  (type $s (struct))
  (import "" "f" (func $f))
  (import "" "g" (func $g))
  (func (param anyref)
    block (result anyref)
      (br_on_cast_fail 0 anyref (ref $s) (local.get 0))
      (call $f)
      return
    end
    (call $g)
    return
  )
)
;; function u0:2(i64 vmctx, i64, i32) tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     sig0 = (i64 vmctx, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) tail
;;     sig2 = (i64 vmctx, i64) tail
;;     fn0 = colocated u1:35 sig0
;;     fn1 = u0:0 sig1
;;     fn2 = u0:1 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v42 = stack_addr.i64 ss0
;;                                     store notrap v2, v42
;;                                     v40 = iconst.i32 0
;; @002e                               v4 = icmp eq v2, v40  ; v40 = 0
;; @002e                               v5 = uextend.i32 v4
;; @002e                               brif v5, block5(v40), block3  ; v40 = 0
;;
;;                                 block3:
;; @002e                               v7 = iconst.i32 1
;; @002e                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v43 = iconst.i32 0
;; @002e                               brif v8, block5(v43), block4  ; v43 = 0
;;
;;                                 block4:
;; @002e                               v36 = load.i64 notrap aligned readonly can_move v0+8
;; @002e                               v14 = load.i64 notrap aligned readonly can_move v36+24
;; @002e                               v13 = uextend.i64 v2
;; @002e                               v15 = iadd v14, v13
;; @002e                               v16 = iconst.i64 4
;; @002e                               v17 = iadd v15, v16  ; v16 = 4
;; @002e                               v18 = load.i32 notrap aligned readonly v17
;; @002e                               v11 = load.i64 notrap aligned readonly can_move v0+40
;; @002e                               v12 = load.i32 notrap aligned readonly can_move v11
;; @002e                               v19 = icmp eq v18, v12
;; @002e                               v20 = uextend.i32 v19
;; @002e                               brif v20, block7(v20), block6
;;
;;                                 block6:
;; @002e                               v22 = call fn0(v0, v18, v12), stack_map=[i32 @ ss0+0]
;; @002e                               jump block7(v22)
;;
;;                                 block7(v23: i32):
;; @002e                               jump block5(v23)
;;
;;                                 block5(v24: i32):
;;                                     v31 = load.i32 notrap v42
;; @002e                               brif v24, block8, block2
;;
;;                                 block8:
;; @0034                               v26 = load.i64 notrap aligned readonly can_move v0+48
;; @0034                               v27 = load.i64 notrap aligned readonly can_move v0+64
;; @0034                               call_indirect sig1, v26(v27, v0)
;; @0036                               return
;;
;;                                 block2:
;; @0038                               v29 = load.i64 notrap aligned readonly can_move v0+72
;; @0038                               v30 = load.i64 notrap aligned readonly can_move v0+88
;; @0038                               call_indirect sig2, v29(v30, v0)
;; @003a                               return
;; }
