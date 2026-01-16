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
;;     gv9 = load.i64 notrap aligned readonly can_move gv7+144
;;     gv10 = load.i64 notrap aligned readonly can_move gv7+96
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
;;                                     v21 = load.i64 notrap aligned readonly can_move v5+72
;;                                     v20 = load.i64 notrap aligned readonly can_move v5+88
;;                                     v19 = iconst.i32 23
;;                                     call_indirect sig1, v21(v20, v5, v19)  ; v19 = 23
;;                                     trap user11
;;
;;                                 block5:
;;                                     v22 = load.i64 notrap aligned readonly can_move v5+144
;;                                     v23 = load.i32 notrap aligned table v22
;;                                     v61 = iconst.i32 0
;;                                     store notrap aligned table v61, v22  ; v61 = 0
;;                                     v26 = load.i64 notrap aligned readonly can_move v5+96
;;                                     v27 = load.i32 notrap aligned table v26
;;                                     v28 = iconst.i32 -2
;;                                     v29 = band v27, v28  ; v28 = -2
;;                                     store notrap aligned table v29, v26
;;                                     v62 = iconst.i32 1
;;                                     v63 = bor v27, v62  ; v62 = 1
;;                                     store notrap aligned table v63, v26
;;                                     jump block6
;;
;;                                 block6:
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block8
;;
;;                                 block8:
;;                                     v40 = load.i32 notrap aligned table v12
;;                                     v64 = iconst.i32 -2
;;                                     v65 = band v40, v64  ; v64 = -2
;;                                     store notrap aligned table v65, v12
;;                                     v66 = iconst.i32 1
;;                                     v67 = bor v40, v66  ; v66 = 1
;;                                     store notrap aligned table v67, v12
;;                                     store.i32 notrap aligned table v23, v22
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block9
;;
;;                                 block9:
;; @00f0                               jump block1
;;
;;                                 block1:
;;                                     v52 = iconst.i32 1276
;; @00f0                               return v52  ; v52 = 1276
;; }
