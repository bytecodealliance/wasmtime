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
;;     gv8 = load.i64 notrap aligned readonly can_move gv7+144
;;     gv9 = load.i64 notrap aligned readonly can_move gv7+120
;;     gv10 = vmctx
;;     gv11 = load.i64 notrap aligned readonly gv10+8
;;     gv12 = load.i64 notrap aligned gv11+16
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i32, i32) -> i64 tail
;;     sig2 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig3 = (i64 vmctx, i64, i32, i64) tail
;;     fn0 = colocated u2:0 sig0
;;     fn1 = colocated u0:0 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00ee                               jump block2
;;
;;                                 block2:
;; @00ee                               v5 = load.i64 notrap aligned readonly can_move v0+64
;;                                     v13 = load.i64 notrap aligned readonly can_move v5+144
;;                                     v14 = load.i32 notrap aligned table v13
;;                                     v15 = iconst.i32 1
;;                                     v16 = band v14, v15  ; v15 = 1
;;                                     v12 = iconst.i32 0
;;                                     v18 = icmp eq v16, v12  ; v12 = 0
;;                                     v19 = uextend.i32 v18
;;                                     trapnz v19, user11
;;                                     jump block5
;;
;;                                 block5:
;;                                     v20 = load.i64 notrap aligned readonly can_move v5+120
;;                                     v21 = load.i32 notrap aligned table v20
;;                                     v22 = iconst.i32 2
;;                                     v23 = band v21, v22  ; v22 = 2
;;                                     v79 = iconst.i32 0
;;                                     v80 = icmp eq v23, v79  ; v79 = 0
;;                                     v26 = uextend.i32 v80
;;                                     trapnz v26, user11
;;                                     jump block7
;;
;;                                 block7:
;;                                     v28 = load.i32 notrap aligned table v20
;;                                     v29 = iconst.i32 -3
;;                                     v30 = band v28, v29  ; v29 = -3
;;                                     store notrap aligned table v30, v20
;;                                     v35 = load.i64 notrap aligned readonly can_move v5+72
;;                                     v34 = load.i64 notrap aligned readonly can_move v5+88
;;                                     v81 = iconst.i32 2
;;                                     v82 = iconst.i32 1
;;                                     v36 = call_indirect sig1, v35(v34, v5, v81, v82)  ; v81 = 2, v82 = 1
;;                                     v38 = load.i32 notrap aligned table v20
;;                                     v39 = iconst.i32 -2
;;                                     v40 = band v38, v39  ; v39 = -2
;;                                     store notrap aligned table v40, v20
;;                                     v83 = bor v38, v82  ; v82 = 1
;;                                     store notrap aligned table v83, v20
;;                                     jump block8
;;
;;                                 block8:
;;                                     jump block9
;;
;;                                 block9:
;;                                     jump block10
;;
;;                                 block10:
;;                                     v51 = load.i32 notrap aligned table v13
;;                                     v84 = iconst.i32 -2
;;                                     v85 = band v51, v84  ; v84 = -2
;;                                     store notrap aligned table v85, v13
;;                                     v86 = iconst.i32 1
;;                                     v87 = bor v51, v86  ; v86 = 1
;;                                     store notrap aligned table v87, v13
;;                                     v61 = load.i32 notrap aligned table v20
;;                                     v88 = iconst.i32 2
;;                                     v89 = bor v61, v88  ; v88 = 2
;;                                     store notrap aligned table v89, v20
;;                                     v67 = load.i64 notrap aligned readonly can_move v5+96
;;                                     v66 = load.i64 notrap aligned readonly can_move v5+112
;;                                     call_indirect sig3, v67(v66, v5, v86, v36)  ; v86 = 1
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
