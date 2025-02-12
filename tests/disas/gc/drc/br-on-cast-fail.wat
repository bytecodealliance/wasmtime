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
;;     sig0 = (i64 vmctx, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64) tail
;;     sig2 = (i64 vmctx, i64) tail
;;     fn0 = colocated u1:35 sig0
;;     fn1 = u0:0 sig1
;;     fn2 = u0:1 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;;                                     v41 = stack_addr.i64 ss0
;;                                     store notrap v2, v41
;;                                     v43 = iconst.i32 0
;; @002e                               v4 = icmp eq v2, v43  ; v43 = 0
;; @002e                               v5 = uextend.i32 v4
;; @002e                               brif v5, block5(v43), block3  ; v43 = 0
;;
;;                                 block3:
;; @002e                               v7 = iconst.i32 1
;; @002e                               v8 = band.i32 v2, v7  ; v7 = 1
;;                                     v47 = iconst.i32 0
;; @002e                               brif v8, block5(v47), block4  ; v47 = 0
;;
;;                                 block4:
;; @002e                               v17 = uextend.i64 v2
;; @002e                               v18 = iconst.i64 4
;; @002e                               v19 = uadd_overflow_trap v17, v18, user1  ; v18 = 4
;; @002e                               v21 = uadd_overflow_trap v19, v18, user1  ; v18 = 4
;; @002e                               v16 = load.i64 notrap aligned readonly v0+48
;; @002e                               v22 = icmp ule v21, v16
;; @002e                               trapz v22, user1
;; @002e                               v14 = load.i64 notrap aligned readonly v0+40
;; @002e                               v23 = iadd v14, v19
;; @002e                               v24 = load.i32 notrap aligned readonly v23
;; @002e                               v11 = load.i64 notrap aligned readonly v0+64
;; @002e                               v12 = load.i32 notrap aligned readonly v11
;; @002e                               v25 = icmp eq v24, v12
;; @002e                               v26 = uextend.i32 v25
;; @002e                               brif v26, block7(v26), block6
;;
;;                                 block6:
;; @002e                               v28 = call fn0(v0, v24, v12), stack_map=[i32 @ ss0+0]
;; @002e                               jump block7(v28)
;;
;;                                 block7(v29: i32):
;; @002e                               jump block5(v29)
;;
;;                                 block5(v30: i32):
;;                                     v37 = load.i32 notrap v41
;; @002e                               brif v30, block8, block2
;;
;;                                 block8:
;; @0034                               v32 = load.i64 notrap aligned readonly v0+72
;; @0034                               v33 = load.i64 notrap aligned readonly v0+88
;; @0034                               call_indirect sig1, v32(v33, v0)
;; @0036                               return
;;
;;                                 block2:
;; @0038                               v35 = load.i64 notrap aligned readonly v0+96
;; @0038                               v36 = load.i64 notrap aligned readonly v0+112
;; @0038                               call_indirect sig2, v35(v36, v0)
;; @003a                               return
;; }
