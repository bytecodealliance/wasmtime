;;! target = "x86_64"

(module
  (data $passive "this is a passive data segment")
  (memory 0)

  (func (export "init") (param i32 i32 i32)
    local.get 0 ;; dst
    local.get 1 ;; src
    local.get 2 ;; cnt
    memory.init $passive)

  (func (export "drop")
    data.drop $passive))

;; function u0:0(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     region4 = 152 "VMContext+0x98"
;;     region5 = 144 "VMContext+0x90"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i64, i64) tail
;;     fn0 = colocated u805306368:1 sig0
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @003d                               v5 = load.i64 notrap aligned region3 v0+64
;; @003d                               v6 = uextend.i64 v2
;; @003d                               v7 = uextend.i64 v4
;; @003d                               v8 = iconst.i64 1
;; @003d                               v9 = imul v7, v8  ; v8 = 1
;; @003d                               v10 = iadd v6, v9
;; @003d                               v11 = icmp ugt v10, v5
;; @003d                               trapnz v11, heap_oob
;; @003d                               v12 = load.i64 notrap aligned readonly can_move region2 v0+56
;; @003d                               v13 = uextend.i64 v2
;; @003d                               v14 = iconst.i64 1
;; @003d                               v15 = imul v13, v14  ; v14 = 1
;; @003d                               v16 = iadd v12, v15
;; @003d                               v17 = load.i32 notrap aligned region4 v0+152
;; @003d                               v18 = uextend.i64 v17
;; @003d                               v19 = uextend.i64 v3
;; @003d                               v20 = uextend.i64 v4
;; @003d                               v21 = iconst.i64 1
;; @003d                               v22 = imul v20, v21  ; v21 = 1
;; @003d                               v23 = iadd v19, v22
;; @003d                               v24 = icmp ugt v23, v18
;; @003d                               trapnz v24, heap_oob
;; @003d                               v25 = load.i64 notrap aligned region5 v0+144
;; @003d                               v26 = uextend.i64 v3
;; @003d                               v27 = iadd v25, v26
;; @003d                               v28 = uextend.i64 v4
;; @003d                               call fn0(v0, v16, v27, v28)
;; @0041                               jump block1
;;
;;                                 block1:
;; @0041                               return
;; }
;;
;; function u0:1(i64 vmctx, i64) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 152 "VMContext+0x98"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @0044                               v2 = iconst.i32 0
;; @0044                               store notrap aligned region2 v2, v0+152  ; v2 = 0
;; @0047                               jump block1
;;
;;                                 block1:
;; @0047                               return
;; }
