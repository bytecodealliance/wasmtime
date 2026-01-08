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
;;     gv8 = load.i64 notrap aligned readonly can_move gv7+120
;;     gv9 = load.i64 notrap aligned readonly can_move gv7+96
;;     gv10 = load.i64 notrap aligned readonly can_move gv7+144
;;     gv11 = vmctx
;;     gv12 = load.i64 notrap aligned readonly gv11+8
;;     gv13 = load.i64 notrap aligned gv12+16
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     sig1 = (i64 vmctx, i64, i32) tail
;;     sig2 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u2:0 sig0
;;     fn1 = colocated u0:0 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00ee                               jump block2
;;
;;                                 block2:
;; @00ee                               v5 = load.i64 notrap aligned readonly can_move v0+64
;;                                     v12 = load.i64 notrap aligned readonly can_move v5+120
;;                                     v13 = load.i32 notrap aligned table v12
;;                                     v14 = iconst.i32 1
;;                                     v15 = band v13, v14  ; v14 = 1
;;                                     v11 = iconst.i32 0
;;                                     v17 = icmp eq v15, v11  ; v11 = 0
;;                                     v18 = uextend.i32 v17
;;                                     brif v18, block4, block5
;;
;;                                 block4:
;;                                     v92 = load.i64 notrap aligned readonly can_move v5+72
;;                                     v93 = load.i64 notrap aligned readonly can_move v5+88
;;                                     v19 = iconst.i32 24
;;                                     call_indirect sig1, v92(v93, v5, v19)  ; v19 = 24
;;                                     trap user11
;;
;;                                 block5:
;;                                     v22 = load.i64 notrap aligned readonly can_move v5+96
;;                                     v23 = load.i32 notrap aligned table v22
;;                                     v24 = iconst.i32 2
;;                                     v25 = band v23, v24  ; v24 = 2
;;                                     v81 = iconst.i32 0
;;                                     v82 = icmp eq v25, v81  ; v81 = 0
;;                                     v28 = uextend.i32 v82
;;                                     brif v28, block6, block7
;;
;;                                 block6:
;;                                     v21 = load.i64 notrap aligned readonly can_move v5+72
;;                                     v20 = load.i64 notrap aligned readonly can_move v5+88
;;                                     v29 = iconst.i32 18
;;                                     call_indirect sig1, v21(v20, v5, v29)  ; v29 = 18
;;                                     trap user11
;;
;;                                 block7:
;;                                     v34 = iconst.i32 -3
;;                                     v35 = band.i32 v23, v34  ; v34 = -3
;;                                     store notrap aligned table v35, v22
;;                                     v37 = load.i64 notrap aligned readonly can_move v5+144
;;                                     v38 = load.i32 notrap aligned table v37
;;                                     v83 = iconst.i32 0
;;                                     store notrap aligned table v83, v37  ; v83 = 0
;;                                     v42 = load.i32 notrap aligned table v22
;;                                     v43 = iconst.i32 -2
;;                                     v44 = band v42, v43  ; v43 = -2
;;                                     store notrap aligned table v44, v22
;;                                     v84 = iconst.i32 1
;;                                     v85 = bor v42, v84  ; v84 = 1
;;                                     store notrap aligned table v85, v22
;;                                     jump block8
;;
;;                                 block8:
;;                                     jump block9
;;
;;                                 block9:
;;                                     jump block10
;;
;;                                 block10:
;;                                     v55 = load.i32 notrap aligned table v12
;;                                     v86 = iconst.i32 -2
;;                                     v87 = band v55, v86  ; v86 = -2
;;                                     store notrap aligned table v87, v12
;;                                     v88 = iconst.i32 1
;;                                     v89 = bor v55, v88  ; v88 = 1
;;                                     store notrap aligned table v89, v12
;;                                     v65 = load.i32 notrap aligned table v22
;;                                     v90 = iconst.i32 2
;;                                     v91 = bor v65, v90  ; v90 = 2
;;                                     store notrap aligned table v91, v22
;;                                     store.i32 notrap aligned table v38, v37
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block11
;;
;;                                 block11:
;; @00f0                               jump block1
;;
;;                                 block1:
;;                                     v72 = iconst.i32 1276
;; @00f0                               return v72  ; v72 = 1276
;; }
