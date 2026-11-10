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
        (memory (core memory $m "mem"))
        (realloc (core func $m "realloc"))))
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
      (memory (core memory $mem "mem"))
      (realloc (core func $mem "realloc"))))

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
;;     region0 = 123 ""
;;     region1 = 160 ""
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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 215 ""
;;     region3 = 105 ""
;;     region4 = 171 ""
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
;;     region0 = 123 ""
;;     region1 = 160 ""
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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 42 ""
;;     region3 = 75 ""
;;     region4 = 215 ""
;;     region5 = 105 ""
;;     region6 = 184 ""
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
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 57 ""
;;     region3 = 209 ""
;;     region4 = 42 ""
;;     region5 = 75 ""
;;     region6 = 215 ""
;;     region7 = 105 ""
;;     region8 = 171 ""
;;     region9 = 184 ""
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
;; @0105                               v10 = load.i64 notrap aligned readonly can_move region4 v0+120
;; @0105                               try_call fn0(v10, v0, v2), sig1, block9(ret0), [ context v0, default: block6(exn0) ]
;;
;;                                 block9(v11: i32):
;; @010b                               v12 = iconst.i32 3
;; @010d                               v13 = band v11, v12  ; v12 = 3
;; @010e                               trapnz v13, user36
;; @010e                               jump block11
;;
;;                                 block11:
;; @0118                               v15 = load.i64 notrap aligned readonly can_move region5 v0+48
;; @0118                               v16 = load.i64 notrap aligned region7 v15+8
;; @0118                               v17 = iconst.i64 16
;; @0118                               v18 = ushr v16, v17  ; v17 = 16
;; @0118                               v19 = ireduce.i32 v18
;; @011a                               v20 = uextend.i64 v19
;; @011d                               v22 = ishl v20, v17  ; v17 = 16
;; @0120                               v23 = uextend.i64 v11
;;                                     v71 = iconst.i64 8
;; @0124                               v26 = iadd v23, v71  ; v71 = 8
;; @0125                               v27 = icmp uge v22, v26
;; @0126                               brif v27, block12, block14
;;
;;                                 block14:
;; @0128                               jump block13
;;
;;                                 block13:
;; @0129                               trap user4
;;
;;                                 block12:
;; @010b                               v72 = iconst.i32 3
;; @0131                               v73 = band.i32 v3, v72  ; v72 = 3
;; @0132                               trapnz v73, user36
;; @0132                               jump block16
;;
;;                                 block16:
;; @013c                               v33 = load.i64 notrap aligned readonly can_move region5 v0+72
;; @013c                               v34 = load.i64 notrap aligned region7 v33+8
;; @0118                               v74 = iconst.i64 16
;; @013c                               v75 = ushr v34, v74  ; v74 = 16
;; @013c                               v37 = ireduce.i32 v75
;; @013e                               v38 = uextend.i64 v37
;; @0141                               v76 = ishl v38, v74  ; v74 = 16
;; @0144                               v41 = uextend.i64 v3
;;                                     v77 = iconst.i64 8
;; @0148                               v78 = iadd v41, v77  ; v77 = 8
;; @0149                               v45 = icmp uge v76, v78
;; @014a                               brif v45, block17, block19
;;
;;                                 block19:
;; @014c                               jump block18
;;
;;                                 block18:
;; @014d                               trap user4
;;
;;                                 block17:
;; @0155                               v50 = load.i64 notrap aligned readonly can_move region6 v15
;; @0155                               v51 = iadd v50, v23
;; @0155                               v52 = load.i32 little region8 v51
;; @0158                               v55 = load.i64 notrap aligned readonly can_move region6 v33
;; @0158                               v56 = iadd v55, v41
;; @0158                               store little region9 v52, v56
;; @0160                               v61 = iconst.i64 4
;; @0160                               v62 = iadd v51, v61  ; v61 = 4
;; @0160                               v63 = load.i32 little region8 v62
;; @0163                               v69 = iadd v56, v61  ; v61 = 4
;; @0163                               store little region9 v63, v69
;; @0167                               jump block5
;;
;;                                 block5:
;; @0168                               jump block2
;;
;;                                 block3:
;; @016b                               trap user52
;;
;;                                 block2:
;; @016f                               jump block1
;;
;;                                 block1:
;; @016f                               return
;; }
