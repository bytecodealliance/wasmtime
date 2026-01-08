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
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+120
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+96
;;     gv6 = load.i64 notrap aligned readonly can_move gv3+144
;;     sig0 = (i64 vmctx, i64, i32) tail
;;     sig1 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0092                               v5 = load.i64 notrap aligned readonly can_move v0+120
;; @0092                               v6 = load.i32 notrap aligned table v5
;; @0094                               v7 = iconst.i32 1
;; @0096                               v8 = band v6, v7  ; v7 = 1
;; @0090                               v4 = iconst.i32 0
;; @0097                               v9 = icmp eq v8, v4  ; v4 = 0
;; @0097                               v10 = uextend.i32 v9
;; @0098                               brif v10, block2, block3
;;
;;                                 block2:
;;                                     v94 = load.i64 notrap aligned readonly can_move v0+72
;;                                     v95 = load.i64 notrap aligned readonly can_move v0+88
;; @009a                               v11 = iconst.i32 24
;; @009c                               call_indirect sig0, v94(v95, v0, v11)  ; v11 = 24
;; @009e                               trap user11
;;
;;                                 block3:
;; @00a0                               v15 = load.i64 notrap aligned readonly can_move v0+96
;; @00a0                               v16 = load.i32 notrap aligned table v15
;; @00a2                               v17 = iconst.i32 2
;; @00a4                               v18 = band v16, v17  ; v17 = 2
;;                                     v86 = iconst.i32 0
;;                                     v87 = icmp eq v18, v86  ; v86 = 0
;; @00a5                               v20 = uextend.i32 v87
;; @00a6                               brif v20, block4, block5
;;
;;                                 block4:
;; @009c                               v14 = load.i64 notrap aligned readonly can_move v0+72
;; @009c                               v13 = load.i64 notrap aligned readonly can_move v0+88
;; @00a8                               v21 = iconst.i32 18
;; @00aa                               call_indirect sig0, v14(v13, v0, v21)  ; v21 = 18
;; @00ac                               trap user11
;;
;;                                 block5:
;; @00b0                               v27 = iconst.i32 -3
;; @00b2                               v28 = band.i32 v16, v27  ; v27 = -3
;; @00b3                               store notrap aligned table v28, v15
;; @00b5                               v30 = load.i64 notrap aligned readonly can_move v0+144
;; @00b5                               v31 = load.i32 notrap aligned table v30
;;                                     v88 = iconst.i32 0
;; @00bb                               store notrap aligned table v88, v30  ; v88 = 0
;; @00bd                               v35 = load.i32 notrap aligned table v15
;; @00bf                               v36 = iconst.i32 -2
;; @00c1                               v37 = band v35, v36  ; v36 = -2
;; @00c2                               store notrap aligned table v37, v15
;;                                     v89 = iconst.i32 1
;;                                     v90 = bor v35, v89  ; v89 = 1
;; @00cb                               store notrap aligned table v90, v15
;; @00cd                               v45 = load.i64 notrap aligned readonly can_move v0+64
;; @00cd                               v46 = call fn0(v45, v0, v2)
;; @00d1                               v48 = load.i32 notrap aligned table v5
;; @00d5                               v50 = band v48, v36  ; v36 = -2
;; @00d6                               store notrap aligned table v50, v5
;;                                     v91 = bor v48, v89  ; v89 = 1
;; @00df                               store notrap aligned table v91, v5
;; @00e1                               v58 = load.i32 notrap aligned table v15
;;                                     v92 = iconst.i32 2
;;                                     v93 = bor v58, v92  ; v92 = 2
;; @00e6                               store notrap aligned table v93, v15
;; @00ea                               store notrap aligned table v31, v30
;; @00ec                               jump block1
;;
;;                                 block1:
;; @00ec                               return v46
;; }
