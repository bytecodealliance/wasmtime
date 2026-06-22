;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[1]--function"
;;! flags = "-C inlining=y -Wconcurrency-support=n"

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
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 72 "VMContext+0x48"
;;     region3 = 136 "VMContext+0x88"
;;     region4 = 1610612736 "PublicGlobal"
;;     region5 = 104 "VMContext+0x68"
;;     region6 = 88 "VMContext+0x58"
;;     region7 = 112 "VMContext+0x70"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned region1 gv4+24
;;     gv6 = vmctx
;;     gv7 = load.i64 notrap aligned readonly can_move region0 gv6+8
;;     gv8 = load.i64 notrap aligned region1 gv7+24
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
;;                                     jump block6
;;
;;                                 block8(v7: i64):
;;                                     jump block5
;;
;;                                 block6:
;; @00ee                               v4 = load.i64 notrap aligned readonly can_move region2 v0+72
;;                                     v12 = load.i64 notrap aligned readonly can_move region3 v4+136
;;                                     v13 = load.i32 notrap aligned region4 v12
;;                                     v14 = iconst.i32 1
;;                                     v15 = band v13, v14  ; v14 = 1
;;                                     v11 = iconst.i32 0
;;                                     v17 = icmp eq v15, v11  ; v11 = 0
;;                                     brif v17, block9, block10
;;
;;                                 block9:
;;                                     v21 = load.i64 notrap aligned readonly can_move region6 v4+88
;;                                     v20 = load.i64 notrap aligned readonly can_move region5 v4+104
;;                                     v19 = iconst.i32 23
;;                                     try_call_indirect v21(v20, v4, v19), sig1, block11, [ context v4, default: block8(exn0) ]  ; v19 = 23
;;
;;                                 block11:
;;                                     trap user12
;;
;;                                 block10:
;;                                     v26 = load.i64 notrap aligned readonly can_move region7 v4+112
;;                                     v27 = load.i32 notrap aligned region4 v26
;;                                     v28 = iconst.i32 -2
;;                                     v29 = band v27, v28  ; v28 = -2
;;                                     store notrap aligned region4 v29, v26
;;                                     v59 = iconst.i32 1
;;                                     v60 = bor v27, v59  ; v59 = 1
;;                                     store notrap aligned region4 v60, v26
;;                                     jump block13
;;
;;                                 block13:
;;                                     jump block14
;;
;;                                 block14:
;;                                     jump block12
;;
;;                                 block12:
;;                                     v40 = load.i32 notrap aligned region4 v12
;;                                     v61 = iconst.i32 -2
;;                                     v62 = band v40, v61  ; v61 = -2
;;                                     store notrap aligned region4 v62, v12
;;                                     v63 = iconst.i32 1
;;                                     v64 = bor v40, v63  ; v63 = 1
;;                                     store notrap aligned region4 v64, v12
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block5:
;;                                     v23 = iconst.i32 49
;;                                     call_indirect.i64 sig1, v21(v20, v4, v23)  ; v23 = 49
;;                                     trap user12
;;
;;                                 block4:
;;                                     jump block3
;;
;;                                 block3:
;;                                     jump block15
;;
;;                                 block15:
;; @00f0                               jump block1
;;
;;                                 block1:
;;                                     v51 = iconst.i32 1276
;; @00f0                               return v51  ; v51 = 1276
;; }
