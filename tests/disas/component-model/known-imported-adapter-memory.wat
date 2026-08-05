;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "function"
;;! flags = "-C inlining=n -Wconcurrency-support=n"

;; Every access of memory contents below (in the modules that define and import
;; the memories, and in the adapter that copies the returned tuple from one to
;; the other) should use the same `DefinedMemory` region rather than the
;; conservative `PublicMemory` region.

(component
  (component $A
    (core module $M
      (memory (export "mem") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (i32.const 0))
      (func (export "f") (param i32) (result i32)
        (i32.store (i32.const 8) (local.get 0))
        (i32.store offset=4 (i32.const 8) (local.get 0))
        (i32.const 8))
    )
    (core instance $m (instantiate $M))
    (func (export "f") (param "a" u32) (result (tuple u32 u32))
      (canon lift (core func $m "f")
        (memory $m "mem")
        (realloc (func $m "realloc"))))
  )

  (instance $a (instantiate $A))

  (component $B
    (import "f" (func $f (param "a" u32) (result (tuple u32 u32))))

    (core module $Mem
      (memory (export "mem") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32)
        (i32.const 0))
    )
    (core instance $mem (instantiate $Mem))

    (core func $f' (canon lower (func $f)
      (memory $mem "mem")
      (realloc (func $mem "realloc"))))

    (core module $N
      (import "" "mem" (memory 1))
      (import "" "f'" (func $f' (param i32 i32)))
      (func (export "g") (result i32)
        (call $f' (i32.const 42) (i32.const 0))
        (i32.load (i32.const 0)))
    )
    (core instance $n (instantiate $N
      (with "" (instance
        (export "mem" (memory $mem "mem"))
        (export "f'" (func $f'))
      ))
    ))
  )

  (instance $b (instantiate $B (with "f" (func $a "f"))))
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @0055                               jump block1
;;
;;                                 block1:
;; @0053                               v6 = iconst.i32 0
;; @0055                               return v6  ; v6 = 0
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 603979776 "VMMemoryDefinition+0x0"
;;     region3 = 603979784 "VMMemoryDefinition+0x8"
;;     region4 = 201326592 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @005c                               v5 = load.i64 notrap aligned readonly can_move region2 v0+56
;;                                     v14 = iconst.i64 8
;; @005c                               v6 = iadd v5, v14  ; v14 = 8
;; @005c                               store little region4 v2, v6
;;                                     v16 = iconst.i64 12
;;                                     v21 = iadd v5, v16  ; v16 = 12
;; @0063                               store little region4 v2, v21
;; @0068                               jump block1
;;
;;                                 block1:
;; @0058                               v3 = iconst.i32 8
;; @0068                               return v3  ; v3 = 8
;; }
;;
;; function u1:0(i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32):
;; @013e                               jump block1
;;
;;                                 block1:
;; @013c                               v6 = iconst.i32 0
;; @013e                               return v6  ; v6 = 0
;; }
;;
;; function u2:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 1275068416 "VMMemoryImport+0x0"
;;     region4 = 603979776 "VMMemoryDefinition+0x0"
;;     region5 = 603979784 "VMMemoryDefinition+0x8"
;;     region6 = 201588736 "DefinedMemory(StaticModuleIndex(1), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32, i32) tail
;;     fn0 = colocated u3:0 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @01af                               v4 = load.i64 notrap aligned readonly can_move region2 v0+96
;; @01ab                               v2 = iconst.i32 42
;; @01ad                               v3 = iconst.i32 0
;; @01af                               call fn0(v4, v0, v2, v3)  ; v2 = 42, v3 = 0
;; @01b3                               v7 = load.i64 notrap aligned readonly can_move region3 v0+48
;; @01b3                               v8 = load.i64 notrap aligned readonly can_move region4 v7
;; @01b3                               v10 = load.i32 little region6 v8
;; @01b6                               jump block1
;;
;;                                 block1:
;; @01b6                               return v10
;; }
;;
;; function u3:0(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1476395008 "VMGlobalImport+0x0"
;;     region3 = 402653184 "PublicGlobal"
;;     region4 = 1207959576 "VMFunctionImport+0x18"
;;     region5 = 1275068416 "VMMemoryImport+0x0"
;;     region6 = 603979776 "VMMemoryDefinition+0x0"
;;     region7 = 603979784 "VMMemoryDefinition+0x8"
;;     region8 = 201326592 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     region9 = 201588736 "DefinedMemory(StaticModuleIndex(1), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32) tail
;;     sig1 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @00be                               jump block4
;;
;;                                 block6(v5: i64):
;; @00be                               jump block3
;;
;;                                 block4:
;; @00c5                               v7 = load.i64 notrap aligned readonly can_move region2 v0+248
;; @00c5                               v8 = load.i32 notrap aligned region3 v7
;; @00c9                               trapz v8, user26
;; @00c9                               jump block7
;;
;;                                 block7:
;; @00d1                               v11 = load.i64 notrap aligned readonly can_move region2 v0+224
;; @00d1                               v12 = load.i32 notrap aligned region3 v11
;; @00df                               v16 = load.i64 notrap aligned readonly can_move region4 v0+184
;; @00df                               try_call fn0(v16, v0, v2), sig1, block9(ret0), [ context v0, default: block6(exn0) ]
;;
;;                                 block9(v17: i32):
;; @00b8                               v4 = iconst.i32 0
;; @00e5                               store notrap aligned region3 v4, v7  ; v4 = 0
;; @00e9                               v20 = iconst.i32 3
;; @00eb                               v21 = band v17, v20  ; v20 = 3
;; @00ec                               trapnz v21, user36
;; @00ec                               jump block11
;;
;;                                 block11:
;; @00f8                               v24 = load.i64 notrap aligned readonly can_move region5 v0+48
;; @00f8                               v25 = load.i64 notrap aligned region7 v24+8
;; @00f8                               v26 = iconst.i64 16
;; @00f8                               v27 = ushr v25, v26  ; v26 = 16
;; @00f8                               v28 = ireduce.i32 v27
;; @00fa                               v29 = uextend.i64 v28
;; @00fd                               v31 = ishl v29, v26  ; v26 = 16
;; @0100                               v32 = uextend.i64 v17
;;                                     v85 = iconst.i64 8
;; @0104                               v35 = iadd v32, v85  ; v85 = 8
;; @0105                               v36 = icmp uge v31, v35
;; @0106                               brif v36, block12, block14
;;
;;                                 block14:
;; @0108                               jump block13
;;
;;                                 block13:
;; @010b                               trap user4
;;
;;                                 block12:
;;                                     v86 = iconst.i32 3
;;                                     v87 = band.i32 v3, v86  ; v86 = 3
;; @0114                               trapnz v87, user36
;; @0114                               jump block16
;;
;;                                 block16:
;; @0120                               v44 = load.i64 notrap aligned readonly can_move region5 v0+72
;; @0120                               v45 = load.i64 notrap aligned region7 v44+8
;;                                     v88 = iconst.i64 16
;;                                     v89 = ushr v45, v88  ; v88 = 16
;; @0120                               v48 = ireduce.i32 v89
;; @0122                               v49 = uextend.i64 v48
;;                                     v90 = ishl v49, v88  ; v88 = 16
;; @0128                               v52 = uextend.i64 v3
;;                                     v91 = iconst.i64 8
;;                                     v92 = iadd v52, v91  ; v91 = 8
;; @012d                               v56 = icmp uge v90, v92
;; @012e                               brif v56, block17, block19
;;
;;                                 block19:
;; @0130                               jump block18
;;
;;                                 block18:
;; @0133                               trap user4
;;
;;                                 block17:
;; @013b                               v62 = load.i64 notrap aligned readonly can_move region6 v24
;; @013b                               v63 = iadd v62, v32
;; @013b                               v64 = load.i32 little region8 v63
;; @013e                               v67 = load.i64 notrap aligned readonly can_move region6 v44
;; @013e                               v68 = iadd v67, v52
;; @013e                               store little region9 v64, v68
;; @0146                               v73 = iconst.i64 4
;; @0146                               v74 = iadd v63, v73  ; v73 = 4
;; @0146                               v75 = load.i32 little region8 v74
;; @0149                               v81 = iadd v68, v73  ; v73 = 4
;; @0149                               store little region9 v75, v81
;; @014f                               store.i32 notrap aligned region3 v8, v7
;; @0151                               jump block5
;;
;;                                 block5:
;; @0152                               jump block2
;;
;;                                 block3:
;; @0157                               trap user52
;;
;;                                 block2:
;; @015b                               jump block1
;;
;;                                 block1:
;; @015b                               return
;; }
