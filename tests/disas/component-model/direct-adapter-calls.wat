;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "function"
;;! flags = "-C inlining=n -Wconcurrency-support=n"

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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+120
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+96
;;     sig0 = (i64 vmctx, i64, i32) tail
;;     sig1 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0077                               v5 = load.i64 notrap aligned readonly can_move v0+120
;; @0077                               v6 = load.i32 notrap aligned table v5
;; @0079                               v7 = iconst.i32 1
;; @007b                               v8 = band v6, v7  ; v7 = 1
;; @0075                               v4 = iconst.i32 0
;; @007c                               v9 = icmp eq v8, v4  ; v4 = 0
;; @007c                               v10 = uextend.i32 v9
;; @007d                               brif v10, block2, block3
;;
;;                                 block2:
;; @0081                               v14 = load.i64 notrap aligned readonly can_move v0+72
;; @0081                               v13 = load.i64 notrap aligned readonly can_move v0+88
;; @007f                               v11 = iconst.i32 23
;; @0081                               call_indirect sig0, v14(v13, v0, v11)  ; v11 = 23
;; @0083                               trap user11
;;
;;                                 block3:
;; @0085                               v15 = load.i64 notrap aligned readonly can_move v0+96
;; @0085                               v16 = load.i32 notrap aligned table v15
;; @0087                               v17 = iconst.i32 -2
;; @0089                               v18 = band v16, v17  ; v17 = -2
;; @008a                               store notrap aligned table v18, v15
;;                                     v52 = iconst.i32 1
;;                                     v53 = bor v16, v52  ; v52 = 1
;; @0093                               store notrap aligned table v53, v15
;; @0095                               v26 = load.i64 notrap aligned readonly can_move v0+64
;; @0095                               v27 = call fn0(v26, v0, v2)
;; @0099                               v29 = load.i32 notrap aligned table v5
;; @009d                               v31 = band v29, v17  ; v17 = -2
;; @009e                               store notrap aligned table v31, v5
;;                                     v54 = bor v29, v52  ; v52 = 1
;; @00a7                               store notrap aligned table v54, v5
;; @00a9                               jump block1
;;
;;                                 block1:
;; @00a9                               return v27
;; }
