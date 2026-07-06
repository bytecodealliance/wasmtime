;;! target = "x86_64"
;;! flags = "-W function-references,gc"
;;! test = "optimize"

(module
  (import "" "observe" (func $observe (param anyref)))
  (import "" "safepoint" (func $safepoint))

  (func (param structref i31ref i32)
    ;; Select from two types, one of which requires inclusion in stack maps,
    ;; resulting in a value that also requires inclusion in stack maps.
    (select (result anyref)
            (local.get 0)
            (local.get 1)
            (local.get 2))

    ;; Make a call, which is a safepoint and has stack maps.
    (call $safepoint)

    ;; Observe the result of the select to keep it alive across the call.
    call $observe
  )

  (func (param i31ref i31ref i32)
        ;; Select from two types that do not require inclusion in stack maps,
        ;; resulting in one that (normally) does. In this case, however, we
        ;; shouldn't include the value in a stack map, because we know that the
        ;; anyref cannot be an instance of a subtype that actually does require
        ;; inclusion in stack maps.
        (select (result anyref)
                (local.get 0)
                (local.get 1)
                (local.get 2))

        ;; Make a call, which is a safepoint and has stack maps.
        (call $safepoint)

        ;; Observe the result of the select to keep it alive across the call.
        call $observe
        )
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) tail {
;;     ss0 = explicit_slot 4, align = 4
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 1207959560 "VMFunctionImport+0x8"
;;     region4 = 1543503872 "Stack(ss0)"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i64, i32) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @0049                               v5 = select v4, v2, v3
;;                                     v12 = stack_addr.i64 ss0
;;                                     store notrap aligned region4 v5, v12
;; @004c                               v7 = load.i64 notrap aligned readonly can_move region3 v0+88
;; @004c                               v6 = load.i64 notrap aligned readonly can_move region2 v0+104
;; @004c                               call_indirect sig0, v7(v6, v0), stack_map=[i32 @ ss0+0]
;;                                     v11 = load.i32 notrap aligned region4 v12
;; @004e                               v9 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @004e                               v8 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @004e                               call_indirect sig1, v9(v8, v0, v11)
;; @0050                               jump block1
;;
;;                                 block1:
;; @0050                               return
;; }
;;
;; function u0:1(i64 vmctx, i64, i32, i32, i32) tail {
;;     region0 = 8 "VMContext+0x8"
;;     region1 = 67108888 "VMStoreContext+0x18"
;;     region2 = 1207959576 "VMFunctionImport+0x18"
;;     region3 = 1207959560 "VMFunctionImport+0x8"
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i64, i32) tail
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;; @005c                               v7 = load.i64 notrap aligned readonly can_move region3 v0+88
;; @005c                               v6 = load.i64 notrap aligned readonly can_move region2 v0+104
;; @005c                               call_indirect sig0, v7(v6, v0)
;; @005e                               v9 = load.i64 notrap aligned readonly can_move region3 v0+56
;; @005e                               v8 = load.i64 notrap aligned readonly can_move region2 v0+72
;; @0059                               v5 = select v4, v2, v3
;; @005e                               call_indirect sig1, v9(v8, v0, v5)
;; @0060                               jump block1
;;
;;                                 block1:
;; @0060                               return
;; }
