;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (param f32 i32 anyref) (result (ref $ty))
    (struct.new $ty (local.get 0) (local.get 1) (local.get 2))
  )
)
;; function u0:0(i64 vmctx, i64, f32, i32, i32) -> i32 tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 872415232 "VMCopyingHeapData+0x0"
;;     region4 = 872415236 "VMCopyingHeapData+0x4"
;;     region5 = 40 "VMContext+0x28"
;;     region6 = 1677721600 "TypeIdsArray+0x0"
;;     region7 = 67108896 "VMStoreContext+0x20"
;;     region8 = 536870912 "GcHeap"
;;     region9 = 1543503872 "Stack(ss0)"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: f32, v3: i32, v4: i32):
;;                                     v47 = stack_addr.i64 ss0
;;                                     store notrap aligned region9 v4, v47
;; @002a                               v6 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @002a                               v7 = load.i32 notrap aligned region3 v6
;; @002a                               v8 = load.i32 notrap aligned region4 v6+4
;; @002a                               v14 = uextend.i64 v7
;;                                     v48 = iconst.i64 32
;; @002a                               v15 = iadd v14, v48  ; v48 = 32
;; @002a                               v16 = uextend.i64 v8
;; @002a                               v17 = icmp ule v15, v16
;; @002a                               brif v17, block2, block3
;;
;;                                 block2:
;;                                     v64 = iconst.i32 32
;;                                     v62 = iadd.i32 v7, v64  ; v64 = 32
;; @002a                               store notrap aligned region3 v62, v6
;;                                     v65 = iconst.i32 -1342177246
;;                                     v66 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v67 = load.i64 notrap aligned readonly can_move region7 v66+32
;; @002a                               v31 = iadd v67, v14
;; @002a                               store user2 region8 v65, v31  ; v65 = -1342177246
;;                                     v68 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v69 = load.i32 notrap aligned readonly can_move region6 v68
;; @002a                               store user2 region8 v69, v31+4
;; @002a                               store user2 region8 v64, v31+8  ; v64 = 32
;; @002a                               jump block4(v7, v31)
;;
;;                                 block3 cold:
;; @002a                               v18 = iconst.i32 -1342177246
;; @002a                               v19 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @002a                               v20 = load.i32 notrap aligned readonly can_move region6 v19
;; @002a                               v5 = iconst.i32 32
;; @002a                               v21 = iconst.i32 16
;; @002a                               v22 = call fn0(v0, v18, v20, v5, v21), stack_map=[i32 @ ss0+0]  ; v18 = -1342177246, v5 = 32, v21 = 16
;; @002a                               v23 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @002a                               v24 = load.i64 notrap aligned readonly can_move region7 v23+32
;; @002a                               v25 = uextend.i64 v22
;; @002a                               v26 = iadd v24, v25
;; @002a                               jump block4(v22, v26)
;;
;;                                 block4(v36: i32, v37: i64):
;; @002a                               v38 = iconst.i64 16
;; @002a                               v39 = iadd v37, v38  ; v38 = 16
;; @002a                               store.f32 user2 little region8 v2, v39
;; @002a                               v42 = ireduce.i8 v3
;; @002a                               v40 = iconst.i64 20
;; @002a                               v41 = iadd v37, v40  ; v40 = 20
;; @002a                               store user2 little region8 v42, v41
;;                                     v46 = load.i32 notrap aligned region9 v47
;; @002a                               v43 = iconst.i64 24
;; @002a                               v44 = iadd v37, v43  ; v43 = 24
;; @002a                               store user2 little region8 v46, v44
;; @002d                               jump block1
;;
;;                                 block1:
;; @002d                               return v36
;; }
