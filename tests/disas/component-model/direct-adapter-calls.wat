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
;; function u0:1(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig0
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
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move gv3+96
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+72
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0064                               v5 = load.i64 notrap aligned readonly can_move v0+96
;; @0064                               v6 = load.i32 notrap aligned table v5
;; @0066                               v7 = iconst.i32 1
;; @0068                               v8 = band v6, v7  ; v7 = 1
;; @0062                               v4 = iconst.i32 0
;; @0069                               v9 = icmp eq v8, v4  ; v4 = 0
;; @0069                               v10 = uextend.i32 v9
;; @006c                               trapnz v10, user11
;; @006a                               jump block3
;;
;;                                 block3:
;; @006e                               v11 = load.i64 notrap aligned readonly can_move v0+72
;; @006e                               v12 = load.i32 notrap aligned table v11
;; @0070                               v13 = iconst.i32 2
;; @0072                               v14 = band v12, v13  ; v13 = 2
;;                                     v79 = iconst.i32 0
;;                                     v80 = icmp eq v14, v79  ; v79 = 0
;; @0073                               v16 = uextend.i32 v80
;; @0076                               trapnz v16, user11
;; @0074                               jump block5
;;
;;                                 block5:
;; @0078                               v18 = load.i32 notrap aligned table v11
;; @007a                               v19 = iconst.i32 -3
;; @007c                               v20 = band v18, v19  ; v19 = -3
;; @007d                               store notrap aligned table v20, v11
;;                                     v67 = iconst.i32 -4
;;                                     v73 = band v18, v67  ; v67 = -4
;; @0084                               store notrap aligned table v73, v11
;;                                     v81 = iconst.i32 1
;;                                     v82 = bor v20, v81  ; v81 = 1
;; @008d                               store notrap aligned table v82, v11
;; @008f                               v33 = load.i64 notrap aligned readonly can_move v0+64
;; @008f                               v34 = call fn0(v33, v0, v2)
;; @0093                               v36 = load.i32 notrap aligned table v5
;; @0081                               v24 = iconst.i32 -2
;; @0097                               v38 = band v36, v24  ; v24 = -2
;; @0098                               store notrap aligned table v38, v5
;;                                     v83 = bor v36, v81  ; v81 = 1
;; @00a1                               store notrap aligned table v83, v5
;; @00a3                               v46 = load.i32 notrap aligned table v11
;;                                     v84 = iconst.i32 2
;;                                     v85 = bor v46, v84  ; v84 = 2
;; @00a8                               store notrap aligned table v85, v11
;; @00aa                               jump block1
;;
;;                                 block1:
;; @00aa                               return v34
;; }
