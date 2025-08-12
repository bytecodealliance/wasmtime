;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=y"

;; Same as `direct-adapter-calls.wat`, except we have enabled function inlining
;; so all the direct calls should get inlined.

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

;; function u1:0(i64 vmctx, i64) -> i32 tail {
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+16
;;     gv3 = vmctx
;;     gv4 = vmctx
;;     gv5 = load.i64 notrap aligned readonly gv4+8
;;     gv6 = load.i64 notrap aligned gv5+16
;;     gv7 = vmctx
;;     gv8 = load.i64 notrap aligned readonly can_move gv7+96
;;     gv9 = load.i64 notrap aligned readonly can_move gv7+72
;;     gv10 = vmctx
;;     gv11 = load.i64 notrap aligned readonly gv10+8
;;     gv12 = load.i64 notrap aligned gv11+16
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u2:0 sig0
;;     fn1 = colocated u0:0 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00ee                               jump block2
;;
;;                                 block2:
;; @00ee                               v5 = load.i64 notrap aligned readonly can_move v0+64
;;                                     v12 = load.i64 notrap aligned readonly can_move v5+96
;;                                     v13 = load.i32 notrap aligned table v12
;;                                     v14 = iconst.i32 1
;;                                     v15 = band v13, v14  ; v14 = 1
;;                                     v11 = iconst.i32 0
;;                                     v17 = icmp eq v15, v11  ; v11 = 0
;;                                     v18 = uextend.i32 v17
;;                                     trapnz v18, user11
;;                                     jump block5
;;
;;                                 block5:
;;                                     v19 = load.i64 notrap aligned readonly can_move v5+72
;;                                     v20 = load.i32 notrap aligned table v19
;;                                     v21 = iconst.i32 2
;;                                     v22 = band v20, v21  ; v21 = 2
;;                                     v79 = iconst.i32 0
;;                                     v80 = icmp eq v22, v79  ; v79 = 0
;;                                     v25 = uextend.i32 v80
;;                                     trapnz v25, user11
;;                                     jump block7
;;
;;                                 block7:
;;                                     v27 = load.i32 notrap aligned table v19
;;                                     v28 = iconst.i32 -3
;;                                     v29 = band v27, v28  ; v28 = -3
;;                                     store notrap aligned table v29, v19
;;                                     v60 = iconst.i32 -4
;;                                     v66 = band v27, v60  ; v60 = -4
;;                                     store notrap aligned table v66, v19
;;                                     v81 = iconst.i32 1
;;                                     v82 = bor v29, v81  ; v81 = 1
;;                                     store notrap aligned table v82, v19
;;                                     jump block8
;;
;;                                 block8:
;;                                     jump block9
;;
;;                                 block9:
;;                                     jump block10
;;
;;                                 block10:
;;                                     v45 = load.i32 notrap aligned table v12
;;                                     v33 = iconst.i32 -2
;;                                     v47 = band v45, v33  ; v33 = -2
;;                                     store notrap aligned table v47, v12
;;                                     v83 = iconst.i32 1
;;                                     v84 = bor v45, v83  ; v83 = 1
;;                                     store notrap aligned table v84, v12
;;                                     v55 = load.i32 notrap aligned table v19
;;                                     v85 = iconst.i32 2
;;                                     v86 = bor v55, v85  ; v85 = 2
;;                                     store notrap aligned table v86, v19
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block11
;;
;;                                 block11:
;; @00f0                               jump block1
;;
;;                                 block1:
;;                                     v70 = iconst.i32 1276
;; @00f0                               return v70  ; v70 = 1276
;; }
