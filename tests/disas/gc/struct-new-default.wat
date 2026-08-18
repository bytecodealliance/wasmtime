;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))
                    (field (mut v128))))

  (func (result (ref $ty))
    (struct.new_default $ty)
  )
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 872415232 "VMCopyingHeapData+0x0"
;;     region4 = 872415236 "VMCopyingHeapData+0x4"
;;     region5 = 40 "VMContext+0x28"
;;     region6 = 1677721600 "TypeIdsArray+0x0"
;;     region7 = 67108896 "VMStoreContext+0x20"
;;     region8 = 536870912 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     const0 = 0x00000000000000000000000000000000
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0023                               v7 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0023                               v8 = load.i32 notrap aligned region3 v7
;; @0023                               v9 = load.i32 notrap aligned region4 v7+4
;; @0023                               v15 = uextend.i64 v8
;;                                     v48 = iconst.i64 48
;; @0023                               v16 = iadd v15, v48  ; v48 = 48
;; @0023                               v17 = uextend.i64 v9
;; @0023                               v18 = icmp ule v16, v17
;; @0023                               brif v18, block2, block3
;;
;;                                 block2:
;;                                     v65 = iconst.i32 48
;;                                     v62 = iadd.i32 v8, v65  ; v65 = 48
;; @0023                               store notrap aligned region3 v62, v7
;;                                     v66 = iconst.i32 -1342177246
;;                                     v67 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v68 = load.i64 notrap aligned readonly can_move region7 v67+32
;; @0023                               v32 = iadd v68, v15
;; @0023                               store user2 region8 v66, v32  ; v66 = -1342177246
;;                                     v69 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v70 = load.i32 notrap aligned readonly can_move region6 v69
;; @0023                               store user2 region8 v70, v32+4
;; @0023                               store user2 region8 v65, v32+8  ; v65 = 48
;; @0023                               jump block4(v8, v32)
;;
;;                                 block3 cold:
;; @0023                               v19 = iconst.i32 -1342177246
;; @0023                               v20 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0023                               v21 = load.i32 notrap aligned readonly can_move region6 v20
;; @0023                               v6 = iconst.i32 48
;; @0023                               v22 = iconst.i32 16
;; @0023                               v23 = call fn0(v0, v19, v21, v6, v22)  ; v19 = -1342177246, v6 = 48, v22 = 16
;; @0023                               v24 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0023                               v25 = load.i64 notrap aligned readonly can_move region7 v24+32
;; @0023                               v26 = uextend.i64 v23
;; @0023                               v27 = iadd v25, v26
;; @0023                               jump block4(v23, v27)
;;
;;                                 block4(v37: i32, v38: i64):
;; @0023                               v2 = f32const 0.0
;; @0023                               v39 = iconst.i64 16
;; @0023                               v40 = iadd v38, v39  ; v39 = 16
;; @0023                               store user2 little region8 v2, v40  ; v2 = 0.0
;;                                     v64 = iconst.i8 0
;; @0023                               v41 = iconst.i64 20
;; @0023                               v42 = iadd v38, v41  ; v41 = 20
;; @0023                               store user2 little region8 v64, v42  ; v64 = 0
;; @0023                               v3 = iconst.i32 0
;; @0023                               v44 = iconst.i64 24
;; @0023                               v45 = iadd v38, v44  ; v44 = 24
;; @0023                               store user2 little region8 v3, v45  ; v3 = 0
;; @0023                               v5 = vconst.i8x16 const0
;; @0023                               v46 = iconst.i64 32
;; @0023                               v47 = iadd v38, v46  ; v46 = 32
;; @0023                               store user2 little region8 v5, v47  ; v5 = const0
;; @0026                               jump block1
;;
;;                                 block1:
;; @0026                               return v37
;; }
