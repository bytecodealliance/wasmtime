;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=null"
;;! test = "optimize"

(module
  (type $ty (array (mut i64)))

  (func (param (ref $ty) i32 i64 i32)
    (array.fill $ty (local.get 0) (local.get 1) (local.get 2) (local.get 3))
  )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i64, i32) tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+8
;;     gv5 = load.i64 notrap aligned readonly can_move gv4+24
;;     gv6 = load.i64 notrap aligned gv4+32
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i64, v5: i32):
;; @0027                               trapz v2, user16
;; @0027                               v41 = load.i64 notrap aligned readonly can_move v0+8
;; @0027                               v7 = load.i64 notrap aligned readonly can_move v41+24
;; @0027                               v6 = uextend.i64 v2
;; @0027                               v8 = iadd v7, v6
;; @0027                               v9 = iconst.i64 8
;; @0027                               v10 = iadd v8, v9  ; v9 = 8
;; @0027                               v11 = load.i32 notrap aligned readonly v10
;; @0027                               v12 = uadd_overflow_trap v3, v5, user17
;; @0027                               v13 = icmp ugt v12, v11
;; @0027                               trapnz v13, user17
;; @0027                               v15 = uextend.i64 v11
;;                                     v43 = iconst.i64 3
;;                                     v44 = ishl v15, v43  ; v43 = 3
;;                                     v40 = iconst.i64 32
;; @0027                               v17 = ushr v44, v40  ; v40 = 32
;; @0027                               trapnz v17, user1
;;                                     v53 = iconst.i32 3
;;                                     v54 = ishl v11, v53  ; v53 = 3
;; @0027                               v19 = iconst.i32 16
;; @0027                               v20 = uadd_overflow_trap v54, v19, user1  ; v19 = 16
;; @0027                               v24 = uadd_overflow_trap v2, v20, user1
;; @0027                               v25 = uextend.i64 v24
;; @0027                               v27 = iadd v7, v25
;;                                     v60 = ishl v3, v53  ; v53 = 3
;;                                     v62 = iadd v60, v19  ; v19 = 16
;; @0027                               v28 = isub v20, v62
;; @0027                               v29 = uextend.i64 v28
;; @0027                               v30 = isub v27, v29
;;                                     v64 = ishl v5, v53  ; v53 = 3
;; @0027                               v32 = uextend.i64 v64
;;                                     v66 = isub v29, v32
;;                                     v67 = isub v27, v66
;; @0027                               jump block2(v30)
;;
;;                                 block2(v35: i64):
;; @0027                               v36 = icmp eq v35, v67
;; @0027                               brif v36, block4, block3
;;
;;                                 block3:
;; @0027                               store.i64 notrap aligned little v4, v35
;;                                     v68 = iconst.i64 8
;;                                     v69 = iadd.i64 v35, v68  ; v68 = 8
;; @0027                               jump block2(v69)
;;
;;                                 block4:
;; @002a                               jump block1
;;
;;                                 block1:
;; @002a                               return
;; }
