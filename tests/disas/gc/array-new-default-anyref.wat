;;! target = 'x86_64'
;;! test = 'optimize'
;;! flags = '-Wgc'

(module
  (type $a (array (mut anyref)))

  (func $fill (param $len i32) (result (ref $a))
    (array.new_default $a (local.get $len))
  )
)
;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+32
;;     gv6 = load.i64 notrap aligned gv4+40
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u805306368:27 sig0
;;     fn1 = colocated u805306368:5 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @001f                               v6 = uextend.i64 v2
;;                                     v50 = iconst.i64 2
;;                                     v51 = ishl v6, v50  ; v50 = 2
;;                                     v48 = iconst.i64 32
;; @001f                               v8 = ushr v51, v48  ; v48 = 32
;; @001f                               trapnz v8, user18
;; @001f                               v5 = iconst.i32 28
;;                                     v57 = iconst.i32 2
;;                                     v58 = ishl v2, v57  ; v57 = 2
;; @001f                               v10 = uadd_overflow_trap v5, v58, user18  ; v5 = 28
;; @001f                               v12 = iconst.i32 -1476395008
;; @001f                               v14 = load.i64 notrap aligned readonly can_move v0+40
;; @001f                               v15 = load.i32 notrap aligned readonly can_move v14
;; @001f                               v16 = iconst.i32 8
;; @001f                               v17 = call fn0(v0, v12, v15, v10, v16)  ; v12 = -1476395008, v16 = 8
;;                                     v47 = stack_addr.i64 ss0
;;                                     store notrap v17, v47
;; @001f                               v45 = load.i64 notrap aligned readonly can_move v0+8
;; @001f                               v18 = load.i64 notrap aligned readonly can_move v45+32
;; @001f                               v19 = uextend.i64 v17
;; @001f                               v20 = iadd v18, v19
;;                                     v43 = iconst.i64 24
;; @001f                               v21 = iadd v20, v43  ; v43 = 24
;; @001f                               store user2 v2, v21
;; @001f                               v31 = load.i64 notrap aligned v45+40
;; @001f                               v27 = uextend.i64 v10
;; @001f                               v28 = iadd v20, v27
;; @001f                               v32 = iadd v18, v31
;; @001f                               v33 = icmp ugt v28, v32
;; @001f                               trapnz v33, user2
;;                                     v62 = iconst.i64 28
;;                                     v67 = iadd v20, v62  ; v62 = 28
;; @001f                               v4 = iconst.i32 0
;; @001f                               v35 = isub v28, v67
;; @001f                               call fn1(v0, v67, v4, v35), stack_map=[i32 @ ss0+0]  ; v4 = 0
;;                                     v36 = load.i32 notrap v47
;; @0022                               jump block1
;;
;;                                 block1:
;; @0022                               return v36
;; }
