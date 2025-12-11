;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "function"
;;! flags = "-C inlining=n"

;; The following component links two sub-components together and each are only
;; instantiated the once, so we statically know what their core modules'
;; function imports will be, and can emit direct calls to those function imports
;; instead of indirect calls through the imports table. There should be zero
;; `call_indirect`s in the disassembly.

(component
  (component $A
    (core module $M
      (func (export "f'") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 42))
      )
    )

    (core instance $m (instantiate $M))

    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))
    )
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))

    (core func $f' (canon lower (func $f)))

    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (func (export "g'") (result i32)
        (call $f' (i32.const 1234))
      )
    )

    (core instance $n
      (instantiate $N
        (with "" (instance (export "f'" (func $f'))))
      )
    )

    (func (export "g") (result u32)
      (canon lift (core func $n "g'"))
    )
  )

  (instance $a (instantiate $A))
  (instance $b
    (instantiate $B
      (with "f" (func $a "f"))
    )
  )

  (export "g" (func $b "g"))
)

;; function u0:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003b                               jump block1
;;
;;                                 block1:
;; @0038                               v4 = iconst.i32 42
;;                                     v5 = iadd.i32 v2, v4  ; v4 = 42
;; @003b                               return v5
;; }
;;
;; function u1:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u2:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00ee                               v5 = load.i64 notrap aligned readonly can_move v0+64
;; @00eb                               v3 = iconst.i32 1234
;; @00ee                               v6 = call fn0(v5, v0, v3)  ; v3 = 1234
;; @00f0                               jump block1
;;
;;                                 block1:
;; @00f0                               return v6
;; }
;;
;; function u2:0(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+144
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+120
;;     sig0 = (i64 vmctx, i64, i32, i32) -> i64 tail
;;     sig1 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig2 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u0:0 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0094                               v6 = load.i64 notrap aligned readonly can_move v0+144
;; @0094                               v7 = load.i32 notrap aligned table v6
;; @0096                               v8 = iconst.i32 1
;; @0098                               v9 = band v7, v8  ; v8 = 1
;; @0092                               v5 = iconst.i32 0
;; @0099                               v10 = icmp eq v9, v5  ; v5 = 0
;; @0099                               v11 = uextend.i32 v10
;; @009c                               trapnz v11, user11
;; @009a                               jump block3
;;
;;                                 block3:
;; @009e                               v12 = load.i64 notrap aligned readonly can_move v0+120
;; @009e                               v13 = load.i32 notrap aligned table v12
;; @00a0                               v14 = iconst.i32 2
;; @00a2                               v15 = band v13, v14  ; v14 = 2
;;                                     v81 = iconst.i32 0
;;                                     v82 = icmp eq v15, v81  ; v81 = 0
;; @00a3                               v17 = uextend.i32 v82
;; @00a6                               trapnz v17, user11
;; @00a4                               jump block5
;;
;;                                 block5:
;; @00a8                               v19 = load.i32 notrap aligned table v12
;; @00aa                               v20 = iconst.i32 -3
;; @00ac                               v21 = band v19, v20  ; v20 = -3
;; @00ad                               store notrap aligned table v21, v12
;; @00b3                               v27 = load.i64 notrap aligned readonly can_move v0+72
;; @00b3                               v26 = load.i64 notrap aligned readonly can_move v0+88
;;                                     v83 = iconst.i32 2
;;                                     v84 = iconst.i32 1
;; @00b3                               v28 = call_indirect sig0, v27(v26, v0, v83, v84)  ; v83 = 2, v84 = 1
;; @00b7                               v30 = load.i32 notrap aligned table v12
;; @00b9                               v31 = iconst.i32 -2
;; @00bb                               v32 = band v30, v31  ; v31 = -2
;; @00bc                               store notrap aligned table v32, v12
;;                                     v85 = bor v30, v84  ; v84 = 1
;; @00c5                               store notrap aligned table v85, v12
;; @00c7                               v40 = load.i64 notrap aligned readonly can_move v0+64
;; @00c7                               v41 = call fn0(v40, v0, v2)
;; @00cb                               v43 = load.i32 notrap aligned table v6
;; @00cf                               v45 = band v43, v31  ; v31 = -2
;; @00d0                               store notrap aligned table v45, v6
;;                                     v86 = bor v43, v84  ; v84 = 1
;; @00d9                               store notrap aligned table v86, v6
;; @00db                               v53 = load.i32 notrap aligned table v12
;;                                     v87 = bor v53, v83  ; v83 = 2
;; @00e0                               store notrap aligned table v87, v12
;; @00e6                               v60 = load.i64 notrap aligned readonly can_move v0+96
;; @00e6                               v59 = load.i64 notrap aligned readonly can_move v0+112
;; @00e6                               call_indirect sig2, v60(v59, v0, v84, v28)  ; v84 = 1
;; @00e8                               jump block1
;;
;;                                 block1:
;; @00e8                               return v41
;; }
