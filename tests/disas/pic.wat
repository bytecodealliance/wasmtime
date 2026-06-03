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
;;     region0 = 805306368 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+64
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+56
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64):
;; @005e                               v7 = load.i64 notrap aligned readonly can_move v0+56
;;                                     v16 = iconst.i64 0x0010_0000
;; @005e                               v8 = iadd v7, v16  ; v16 = 0x0010_0000
;; @005e                               v9 = load.i32 little region0 v8
;; @0061                               jump block1
;;
;;                                 block1:
;; @0061                               return v9
;; }
;;
;; function u0:1(i64 vmctx, i64, i32) -> i32 tail {
;;     region0 = 1879048192 "DefinedGlobal(StaticModuleIndex(0), DefinedGlobalIndex(0))"
;;     region1 = 805306368 "DefinedMemory(StaticModuleIndex(0), DefinedMemoryIndex(0))"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly gv0+8
;;     gv2 = load.i64 notrap aligned gv1+24
;;     gv3 = vmctx
;;     gv4 = load.i64 notrap aligned gv3+64
;;     gv5 = load.i64 notrap aligned readonly can_move gv3+56
;;     sig0 = (i64 vmctx, i64, i32) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @0066                               v6 = load.i32 notrap aligned region0 v0+128
;; @0068                               v7 = iconst.i32 16
;; @006a                               v8 = isub v6, v7  ; v7 = 16
;; @006d                               store notrap aligned region0 v8, v0+128
;; @0073                               v11 = load.i64 notrap aligned readonly can_move v0+56
;; @0073                               v10 = uextend.i64 v2
;; @0073                               v12 = iadd v11, v10
;; @0073                               v13 = load.i32 little region1 v12
;; @0078                               v14 = uextend.i64 v8
;; @0078                               v16 = iadd v11, v14
;; @0078                               v17 = iconst.i64 12
;; @0078                               v18 = iadd v16, v17  ; v17 = 12
;; @0078                               store little region1 v13, v18
;; @0084                               v24 = load.i64 notrap aligned readonly can_move v0+80
;; @0084                               v23 = load.i64 notrap aligned readonly can_move v0+96
;;                                     v36 = iconst.i32 -4
;;                                     v37 = iadd v6, v36  ; v36 = -4
;; @0084                               call_indirect sig0, v24(v23, v0, v37)
;;                                     v44 = iconst.i64 0x0010_0000
;; @0090                               v29 = iadd v11, v44  ; v44 = 0x0010_0000
;; @0090                               store little region1 v13, v29
;; @0098                               store notrap aligned region0 v6, v0+128
;; @009c                               jump block1
;;
;;                                 block1:
;; @009c                               return v13
;; }
