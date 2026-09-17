;;! target = "x86_64"
;;! test = "optimize"
;;! flags = "-W function-references,gc -C collector=null"

;; Unrelated array types get different element regions. These differ only in
;; element mutability, which defeats rec-group deduplication while leaving the
;; regions as the only difference below.

(module
  (type $a (array (mut i32)))
  (type $b (array i32))

  (func $f (param i32) nop)

  (func (param (ref $a)) (param (ref $b))
    ;; The calls make it easy to see which load is the element load in the CLIF,
    ;; so we can check their regions.
    (call $f (array.get $a (local.get 0) (i32.const 0)))
    (call $f (array.get $b (local.get 1) (i32.const 0)))
  )
)
;; function u0:0(i64 vmctx, i64, i32) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0027                               jump block1
;;
;;                                 block1:
;; @0027                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 196 ""
;;     region3 = 206 ""
;;     region4 = 108 ""
;;     region5 = 5 ""
;;     region6 = 214 ""
;;     region7 = 135 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32) tail
;;     fn0 = colocated u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;;                                     v68 = stack_addr.i64 ss0
;;                                     store notrap aligned region7 v3, v68
;; @002e                               trapz v2, user16
;; @002e                               v6 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002e                               v7 = load.i64 notrap aligned readonly can_move region2 v6+32
;; @002e                               v5 = uextend.i64 v2
;; @002e                               v8 = iadd v7, v5
;; @002e                               v9 = iconst.i64 8
;; @002e                               v10 = iadd v8, v9  ; v9 = 8
;; @002e                               v11 = load.i32 user2 readonly region4 v10
;; @002e                               trapz v11, user17
;; @002e                               v14 = uextend.i64 v11
;;                                     v74 = iconst.i64 2
;;                                     v75 = ishl v14, v74  ; v74 = 2
;; @002e                               v16 = iconst.i64 32
;; @002e                               v17 = ushr v75, v16  ; v16 = 32
;; @002e                               trapnz v17, user2
;;                                     v82 = iconst.i32 2
;;                                     v83 = ishl v11, v82  ; v82 = 2
;; @002e                               v19 = iconst.i32 12
;; @002e                               v20 = uadd_overflow_trap v83, v19, user2  ; v19 = 12
;; @002e                               v24 = uadd_overflow_trap v2, v20, user2
;; @002e                               v25 = uextend.i64 v24
;; @002e                               v28 = iadd v7, v25
;; @002e                               v29 = isub v20, v19  ; v19 = 12
;; @002e                               v30 = uextend.i64 v29
;; @002e                               v31 = isub v28, v30
;; @002e                               v32 = load.i32 user2 little region5 v31
;; @0031                               call fn0(v0, v0, v32), stack_map=[i32 @ ss0+0]
;;                                     v67 = load.i32 notrap aligned region7 v68
;; @0037                               trapz v67, user16
;; @0037                               v34 = uextend.i64 v67
;; @0037                               v37 = iadd v7, v34
;; @0037                               v39 = iadd v37, v9  ; v9 = 8
;; @0037                               v40 = load.i32 user2 readonly region4 v39
;; @0037                               trapz v40, user17
;; @0037                               v43 = uextend.i64 v40
;;                                     v104 = ishl v43, v74  ; v74 = 2
;; @0037                               v46 = ushr v104, v16  ; v16 = 32
;; @0037                               trapnz v46, user2
;;                                     v109 = ishl v40, v82  ; v82 = 2
;; @0037                               v49 = uadd_overflow_trap v109, v19, user2  ; v19 = 12
;; @0037                               v53 = uadd_overflow_trap v67, v49, user2
;; @0037                               v54 = uextend.i64 v53
;; @0037                               v57 = iadd v7, v54
;; @0037                               v58 = isub v49, v19  ; v19 = 12
;; @0037                               v59 = uextend.i64 v58
;; @0037                               v60 = isub v57, v59
;; @0037                               v61 = load.i32 user2 little region6 v60
;; @003a                               call fn0(v0, v0, v61)
;; @003c                               jump block1
;;
;;                                 block1:
;; @003c                               return
;; }
