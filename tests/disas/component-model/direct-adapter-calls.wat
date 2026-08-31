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
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @003b                               jump block1
;;
;;                                 block1:
;; @0038                               v3 = iconst.i32 42
;;                                     v4 = iadd.i32 v2, v3  ; v3 = 42
;; @003b                               return v4
;; }
;;
;; function u1:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 239 "VMFunctionImport+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u2:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @00ee                               v3 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @00eb                               v2 = iconst.i32 1234
;; @00ee                               v4 = call fn0(v3, v0, v2)  ; v2 = 1234
;; @00f0                               jump block1
;;
;;                                 block1:
;; @00f0                               return v4
;; }
;;
;; function u2:0(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 15 "VMContext+0x8"
;;     region1 = 114 "VMStoreContext+0x18"
;;     region2 = 190 "VMGlobalImport+0x0"
;;     region3 = 78 "VMComponentContext+0x40"
;;     region4 = 239 "VMFunctionImport+0x18"
;;     region5 = 249 "VMComponentContext+0x30"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @008d                               jump block4
;;
;;                                 block6(v4: i64):
;; @008d                               jump block3
;;
;;                                 block4:
;; @0094                               v6 = load.i64 notrap aligned readonly can_move region2 v0+168
;; @0094                               v7 = load.i32 notrap aligned region3 v6
;; @0098                               trapz v7, user26
;; @0098                               jump block7
;;
;;                                 block7:
;; @009e                               v9 = load.i64 notrap aligned readonly can_move region2 v0+144
;; @009e                               v10 = load.i32 notrap aligned region5 v9
;; @00ac                               v14 = load.i64 notrap aligned readonly can_move region4 v0+72
;; @00ac                               try_call fn0(v14, v0, v2), sig1, block9(ret0), [ context v0, default: block6(exn0) ]
;;
;;                                 block9(v15: i32):
;; @00b8                               store.i32 notrap aligned region3 v7, v6
;; @00ba                               jump block5
;;
;;                                 block5:
;; @00bb                               jump block2
;;
;;                                 block3:
;; @00be                               trap user52
;;
;;                                 block2:
;; @00c2                               jump block1
;;
;;                                 block1:
;; @00c2                               return v15
;; }
