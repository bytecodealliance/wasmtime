;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[0]--function"
;;! flags = "-W function-references,gc -C collector=null -C inlining=y"

;; The inlined `struct.set` lands in the same region as the caller's
;; `struct.get` and is forwarded: `$get_after_set` returns `$val` directly.

(module
  (type $ty (sub (struct (field (mut i32)))))

  (func $set (param (ref $ty)) (param i32)
    (struct.set $ty 0 (local.get 0) (local.get 1))
  )

  (func $get_after_set (param $obj (ref $ty)) (param $val i32) (result i32)
    (call $set (local.get $obj) (local.get $val))
    (struct.get $ty 0 (local.get $obj))
  )
)

;; function u0:0(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 196 ""
;;     region3 = 206 ""
;;     region4 = 147 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @002c                               trapz v2, user16
;; @002c                               v5 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002c                               v6 = load.i64 notrap aligned readonly can_move region2 v5+32
;; @002c                               v4 = uextend.i64 v2
;; @002c                               v7 = iadd v6, v4
;; @002c                               v8 = iconst.i64 8
;; @002c                               v9 = iadd v7, v8  ; v8 = 8
;; @002c                               store user2 little region4 v3, v9
;; @0030                               jump block1
;;
;;                                 block1:
;; @0030                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 196 ""
;;     region3 = 206 ""
;;     region4 = 147 ""
;;     region5 = 135 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     sig0 = (i64 vmctx, i64, i32, i32) tail
;;     fn0 = colocated u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;;                                     v17 = stack_addr.i64 ss0
;;                                     store notrap aligned region5 v2, v17
;; @0037                               jump block2
;;
;;                                 block2:
;;                                     trapz.i32 v2, user16
;;                                     v19 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v20 = load.i64 notrap aligned readonly can_move region2 v19+32
;;                                     v18 = uextend.i64 v2
;;                                     v21 = iadd v20, v18
;;                                     v22 = iconst.i64 8
;;                                     v23 = iadd v21, v22  ; v22 = 8
;;                                     store.i32 user2 little region4 v3, v23
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block4
;;
;;                                 block4:
;; @003f                               jump block1
;;
;;                                 block1:
;; @003f                               return v3
;; }
