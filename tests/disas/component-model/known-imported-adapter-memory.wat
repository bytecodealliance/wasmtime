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
;;     region3 = 738197568 "VMComponentContext+0x40"
;;     region4 = 1207959576 "VMFunctionImport+0x18"
;;     region5 = 738197552 "VMComponentContext+0x30"
;;     region6 = 1275068416 "VMMemoryImport+0x0"
;;     region7 = 603979776 "VMMemoryDefinition+0x0"
;;     region8 = 603979784 "VMMemoryDefinition+0x8"
;;     region9 = 201326592 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     region10 = 201588736 "DefinedMemory(StaticModuleIndex(1), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:1 sig1
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @00f2                               jump block4
;;
;;                                 block6(v5: i64):
;; @00f2                               jump block3
;;
;;                                 block4:
;; @00f9                               v7 = load.i64 notrap aligned readonly can_move region2 v0+344
;; @00f9                               v8 = load.i32 notrap aligned region3 v7
;; @00fd                               trapz v8, user26
;; @00fd                               jump block7
;;
;;                                 block7:
;; @0103                               v10 = load.i64 notrap aligned readonly can_move region2 v0+320
;; @0103                               v11 = load.i32 notrap aligned region5 v10
;; @0111                               v15 = load.i64 notrap aligned readonly can_move region4 v0+184
;; @0111                               try_call fn0(v15, v0, v2), sig1, block9(ret0), [ context v0, default: block6(exn0) ]
;;
;;                                 block9(v16: i32):
;; @00ec                               v4 = iconst.i32 0
;; @0117                               store notrap aligned region3 v4, v7  ; v4 = 0
;; @011b                               v19 = iconst.i32 3
;; @011d                               v20 = band v16, v19  ; v19 = 3
;; @011e                               trapnz v20, user36
;; @011e                               jump block11
;;
;;                                 block11:
;; @0128                               v22 = load.i64 notrap aligned readonly can_move region6 v0+48
;; @0128                               v23 = load.i64 notrap aligned region8 v22+8
;; @0128                               v24 = iconst.i64 16
;; @0128                               v25 = ushr v23, v24  ; v24 = 16
;; @0128                               v26 = ireduce.i32 v25
;; @012a                               v27 = uextend.i64 v26
;; @012d                               v29 = ishl v27, v24  ; v24 = 16
;; @0130                               v30 = uextend.i64 v16
;;                                     v79 = iconst.i64 8
;; @0134                               v33 = iadd v30, v79  ; v79 = 8
;; @0135                               v34 = icmp uge v29, v33
;; @0136                               brif v34, block12, block14
;;
;;                                 block14:
;; @0138                               jump block13
;;
;;                                 block13:
;; @0139                               trap user4
;;
;;                                 block12:
;;                                     v80 = iconst.i32 3
;;                                     v81 = band.i32 v3, v80  ; v80 = 3
;; @0142                               trapnz v81, user36
;; @0142                               jump block16
;;
;;                                 block16:
;; @014c                               v40 = load.i64 notrap aligned readonly can_move region6 v0+72
;; @014c                               v41 = load.i64 notrap aligned region8 v40+8
;;                                     v82 = iconst.i64 16
;;                                     v83 = ushr v41, v82  ; v82 = 16
;; @014c                               v44 = ireduce.i32 v83
;; @014e                               v45 = uextend.i64 v44
;;                                     v84 = ishl v45, v82  ; v82 = 16
;; @0154                               v48 = uextend.i64 v3
;;                                     v85 = iconst.i64 8
;;                                     v86 = iadd v48, v85  ; v85 = 8
;; @0159                               v52 = icmp uge v84, v86
;; @015a                               brif v52, block17, block19
;;
;;                                 block19:
;; @015c                               jump block18
;;
;;                                 block18:
;; @015d                               trap user4
;;
;;                                 block17:
;; @0165                               v57 = load.i64 notrap aligned readonly can_move region7 v22
;; @0165                               v58 = iadd v57, v30
;; @0165                               v59 = load.i32 little region9 v58
;; @0168                               v62 = load.i64 notrap aligned readonly can_move region7 v40
;; @0168                               v63 = iadd v62, v48
;; @0168                               store little region10 v59, v63
;; @0170                               v68 = iconst.i64 4
;; @0170                               v69 = iadd v58, v68  ; v68 = 4
;; @0170                               v70 = load.i32 little region9 v69
;; @0173                               v76 = iadd v63, v68  ; v68 = 4
;; @0173                               store little region10 v70, v76
;; @0179                               store.i32 notrap aligned region3 v8, v7
;; @017b                               jump block5
;;
;;                                 block5:
;; @017c                               jump block2
;;
;;                                 block3:
;; @017f                               trap user52
;;
;;                                 block2:
;; @0183                               jump block1
;;
;;                                 block1:
;; @0183                               return
;; }
