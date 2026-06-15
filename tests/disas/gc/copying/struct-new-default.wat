;;! target = "x86_64"
;;! flags = "-W function-references,gc -C collector=copying"
;;! test = "optimize"
(module
  (type $ty (struct (field (mut f32))
                    (field (mut i8))
                    (field (mut anyref))))

  (func (result (ref $ty))
    (struct.new_default $ty)
  )
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 32 "VMContext+0x20"
;;     region3 = 3489660928 "VMCopyingHeapData+0x0"
;;     region4 = 3489660932 "VMCopyingHeapData+0x4"
;;     region5 = 40 "VMContext+0x28"
;;     region6 = 268435488 "VMStoreContext+0x20"
;;     region7 = 2147483648 "GcHeap"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i32, i32, i32, i32) -> i32 tail
;;     fn0 = colocated u805306368:24 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0021                               v6 = load.i64 notrap aligned readonly can_move region2 v0+32
;; @0021                               v7 = load.i32 notrap aligned region3 v6
;; @0021                               v8 = load.i32 notrap aligned region4 v6+4
;; @0021                               v14 = uextend.i64 v7
;;                                     v43 = iconst.i64 32
;; @0021                               v15 = iadd v14, v43  ; v43 = 32
;; @0021                               v16 = uextend.i64 v8
;; @0021                               v17 = icmp ule v15, v16
;; @0021                               brif v17, block2, block3
;;
;;                                 block2:
;;                                     v59 = iconst.i32 32
;;                                     v57 = iadd.i32 v7, v59  ; v59 = 32
;; @0021                               store notrap aligned region3 v57, v6
;;                                     v60 = iconst.i32 -1342177246
;;                                     v61 = load.i64 notrap aligned readonly can_move region0 v0+8
;;                                     v62 = load.i64 notrap aligned readonly can_move region6 v61+32
;; @0021                               v31 = iadd v62, v14
;; @0021                               store user2 region7 v60, v31  ; v60 = -1342177246
;;                                     v63 = load.i64 notrap aligned readonly can_move region5 v0+40
;;                                     v64 = load.i32 notrap aligned readonly can_move v63
;; @0021                               store user2 region7 v64, v31+4
;;                                     v65 = iconst.i64 32
;; @0021                               istore32 user2 region7 v65, v31+8  ; v65 = 32
;; @0021                               jump block4(v7, v31)
;;
;;                                 block3 cold:
;; @0021                               v18 = iconst.i32 -1342177246
;; @0021                               v19 = load.i64 notrap aligned readonly can_move region5 v0+40
;; @0021                               v20 = load.i32 notrap aligned readonly can_move v19
;; @0021                               v5 = iconst.i32 32
;; @0021                               v21 = iconst.i32 16
;; @0021                               v22 = call fn0(v0, v18, v20, v5, v21)  ; v18 = -1342177246, v5 = 32, v21 = 16
;; @0021                               v23 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @0021                               v24 = load.i64 notrap aligned readonly can_move region6 v23+32
;; @0021                               v25 = uextend.i64 v22
;; @0021                               v26 = iadd v24, v25
;; @0021                               jump block4(v22, v26)
;;
;;                                 block4(v35: i32, v36: i64):
;; @0021                               v2 = f32const 0.0
;; @0021                               v37 = iconst.i64 16
;; @0021                               v38 = iadd v36, v37  ; v37 = 16
;; @0021                               store user2 little region7 v2, v38  ; v2 = 0.0
;; @0021                               v3 = iconst.i32 0
;; @0021                               v39 = iconst.i64 20
;; @0021                               v40 = iadd v36, v39  ; v39 = 20
;; @0021                               istore8 user2 little region7 v3, v40  ; v3 = 0
;; @0021                               v41 = iconst.i64 24
;; @0021                               v42 = iadd v36, v41  ; v41 = 24
;; @0021                               store user2 little region7 v3, v42  ; v3 = 0
;; @0024                               jump block1
;;
;;                                 block1:
;; @0024                               return v35
;; }
