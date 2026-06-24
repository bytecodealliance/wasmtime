;;! target = "x86_64"
;;! test = "optimize"

(module $test.wasm
  (type (;0;) (func (param i32)))
  (type (;1;) (func (result i32)))
  (type (;2;) (func (param i32) (result i32)))
  (type (;3;) (func))
  (import "env" "force_frame" (func $force_frame (;0;) (type 0)))
  (table (;0;) 1 1 funcref)
  (memory (;0;) 17)
  (global $__stack_pointer (;0;) (mut i32) i32.const 1048576)
  (global $GOT.data.internal.__memory_base (;1;) i32 i32.const 0)
  (func $get_var (;1;) (type 1) (result i32)
    global.get $GOT.data.internal.__memory_base
    i32.const 1048576
    i32.add
    i32.load
  )
  (func $set_var (;2;) (type 2) (param i32) (result i32)
    (local i32 i32)
    global.get $__stack_pointer
    i32.const 16
    i32.sub
    local.tee 1
    global.set $__stack_pointer
    local.get 1
    local.get 0
    i32.load
    local.tee 0
    i32.store offset=12
    global.get $GOT.data.internal.__memory_base
    local.set 2
    local.get 1
    i32.const 12
    i32.add
    call $force_frame
    local.get 2
    i32.const 1048576
    i32.add
    local.get 0
    i32.store
    local.get 1
    i32.const 16
    i32.add
    global.set $__stack_pointer
    local.get 0
  )
)
;; function u0:0(i64 vmctx, i64) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 2415919104 "VMMemoryDefinition+0x0"
;;     region3 = 2415919112 "VMMemoryDefinition+0x8"
;;     region4 = 805306368 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @005e                               v6 = load.i64 notrap aligned readonly can_move region2 v0+56
;;                                     v14 = iconst.i64 0x0010_0000
;; @005e                               v7 = iadd v6, v14  ; v14 = 0x0010_0000
;; @005e                               v8 = load.i32 little region4 v7
;; @0061                               jump block1
;;
;;                                 block1:
;; @0061                               return v8
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 268435480 "VMStoreContext+0x18"
;;     region2 = 1879048192 "DefinedGlobal(StaticModuleIndex(0), DefinedGlobalIndex(0))"
;;     region3 = 2415919104 "VMMemoryDefinition+0x0"
;;     region4 = 2415919112 "VMMemoryDefinition+0x8"
;;     region5 = 805306368 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     region6 = 96 "VMContext+0x60"
;;     region7 = 80 "VMContext+0x50"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64, i32) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0066                               v4 = load.i32 notrap aligned region2 v0+128
;; @0068                               v5 = iconst.i32 16
;; @006a                               v6 = isub v4, v5  ; v5 = 16
;; @006d                               store notrap aligned region2 v6, v0+128
;; @0073                               v8 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @0073                               v7 = uextend.i64 v2
;; @0073                               v9 = iadd v8, v7
;; @0073                               v10 = load.i32 little region5 v9
;; @0078                               v11 = uextend.i64 v6
;; @0078                               v13 = iadd v8, v11
;; @0078                               v14 = iconst.i64 12
;; @0078                               v15 = iadd v13, v14  ; v14 = 12
;; @0078                               store little region5 v10, v15
;; @0084                               v20 = load.i64 notrap aligned readonly can_move region7 v0+80
;; @0084                               v19 = load.i64 notrap aligned readonly can_move region6 v0+96
;;                                     v28 = iconst.i32 -4
;;                                     v29 = iadd v4, v28  ; v28 = -4
;; @0084                               call_indirect sig0, v20(v19, v0, v29)
;;                                     v36 = iconst.i64 0x0010_0000
;; @0090                               v25 = iadd v8, v36  ; v36 = 0x0010_0000
;; @0090                               store little region5 v10, v25
;; @0098                               store notrap aligned region2 v4, v0+128
;; @009c                               jump block1
;;
;;                                 block1:
;; @009c                               return v10
;; }
