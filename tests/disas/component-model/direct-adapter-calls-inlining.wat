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
;;     region1 = 72 "VMContext+0x48"
;;     region2 = 136 "VMContext+0x88"
;;     region3 = 112 "VMContext+0x70"
;;     region4 = 1610612736 "PublicGlobal"
;;     region5 = 104 "VMContext+0x68"
;;     region6 = 88 "VMContext+0x58"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned readonly can_move region0 gv3+8
;;     gv5 = load.i64 notrap aligned gv4+24
;;     gv6 = vmctx
;;     gv7 = load.i64 notrap aligned readonly can_move region2 gv6+136
;;     gv8 = load.i64 notrap aligned readonly can_move region3 gv6+112
;;     gv9 = vmctx
;;     gv10 = load.i64 notrap aligned readonly can_move region0 gv9+8
;;     gv11 = load.i64 notrap aligned gv10+24
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
;;                                 block8(v9: i64):
;;                                     jump block5
;;
;;                                 block6:
;; @00ee                               v4 = load.i64 notrap aligned readonly can_move region1 v0+72
;;                                     v14 = load.i64 notrap aligned readonly can_move region2 v4+136
;;                                     v15 = load.i32 notrap aligned region4 v14
;;                                     v16 = iconst.i32 1
;;                                     v17 = band v15, v16  ; v16 = 1
;;                                     v13 = iconst.i32 0
;;                                     v19 = icmp eq v17, v13  ; v13 = 0
;;                                     brif v19, block9, block10
;;
;;                                 block9:
;;                                     v23 = load.i64 notrap aligned readonly can_move region6 v4+88
;;                                     v22 = load.i64 notrap aligned readonly can_move region5 v4+104
;;                                     v21 = iconst.i32 23
;;                                     try_call_indirect v23(v22, v4, v21), sig1, block11, [ context v4, default: block8(exn0) ]  ; v21 = 23
;;
;;                                 block11:
;;                                     trap user12
;;
;;                                 block10:
;;                                     v28 = load.i64 notrap aligned readonly can_move region3 v4+112
;;                                     v29 = load.i32 notrap aligned region4 v28
;;                                     v30 = iconst.i32 -2
;;                                     v31 = band v29, v30  ; v30 = -2
;;                                     store notrap aligned region4 v31, v28
;;                                     v61 = iconst.i32 1
;;                                     v62 = bor v29, v61  ; v61 = 1
;;                                     store notrap aligned region4 v62, v28
;;                                     jump block13
;;
;;                                 block13:
;;                                     jump block14
;;
;;                                 block14:
;;                                     jump block12
;;
;;                                 block12:
;;                                     v42 = load.i32 notrap aligned region4 v14
;;                                     v63 = iconst.i32 -2
;;                                     v64 = band v42, v63  ; v63 = -2
;;                                     store notrap aligned region4 v64, v14
;;                                     v65 = iconst.i32 1
;;                                     v66 = bor v42, v65  ; v65 = 1
;;                                     store notrap aligned region4 v66, v14
;;                                     jump block7
;;
;;                                 block7:
;;                                     jump block4
;;
;;                                 block5:
;;                                     v25 = iconst.i32 49
;;                                     call_indirect.i64 sig1, v23(v22, v4, v25)  ; v25 = 49
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
;;                                     v53 = iconst.i32 1276
;; @00f0                               return v53  ; v53 = 1276
;; }
