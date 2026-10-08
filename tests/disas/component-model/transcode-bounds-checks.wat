;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm-call-component-transcode"

;; The trampolines for the host string transcoders bounds-check both buffers
;; against their linear memories before calling the host, independently of the
;; checks in the adapter module.

(component
  (component $callee
    (core module $m
      (memory (export "memory") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) unreachable)
      (func (export "f") (param i32 i32) (result i32) unreachable)
    )
    (core instance $m (instantiate $m))
    (func (export "f") (param "a" string) (result string)
      (canon lift (core func $m "f")
        (memory (core memory $m "memory"))
        (realloc (core func $m "realloc"))
        string-encoding=utf16))
  )

  (component $caller
    (import "f" (func $f (param "a" string) (result string)))
    (core module $libc
      (memory (export "memory") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) unreachable)
    )
    (core instance $libc (instantiate $libc))
    (core func $f
      (canon lower (func $f)
        (memory (core memory $libc "memory"))
        (realloc (core func $libc "realloc"))
        string-encoding=utf8))
    (core module $m
      (import "" "f" (func (param i32 i32 i32)))
    )
    (core instance (instantiate $m (with "" (instance (export "f" (func $f))))))
  )

  (instance $callee (instantiate $callee))
  (instance (instantiate $caller (with "f" (func $callee "f"))))
)
;; function u0:0(i64 vmctx, i64, i32, i32, i32) -> i32 tail {
;;     region0 = 123 ""
;;     region1 = 85 ""
;;     region2 = 72 ""
;;     region3 = 50 ""
;;     region4 = 105 ""
;;     region5 = 215 ""
;;     region6 = 64 ""
;;     region7 = 245 ""
;;     region8 = 210 ""
;;     region9 = 12 ""
;;     region10 = 184 ""
;;     sig0 = (i64 uext, i64 uext, i64 uext, i64 uext) -> i64 uext system_v
;;     sig1 = (i64 uext vmctx) system_v
;;
;; block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32):
;;     v6 = get_frame_pointer.i64 
;;     v5 = load.i64 notrap aligned readonly can_move region0 v1+8
;;     store notrap aligned region1 v6, v5+48
;;     v7 = get_return_address.i64 
;;     store notrap aligned region2 v7, v5+56
;;     v8 = load.i64 notrap aligned region3 v0+976
;;     v9 = load.i64 notrap aligned region4 v8+8
;;     v10 = uextend.i64 v2
;;     v12 = icmp ugt v10, v9
;;     v11 = uextend.i64 v3
;;     v13 = isub v9, v10
;;     v16 = icmp ugt v11, v13
;;     v17 = bor v12, v16
;;     trapnz v17, user33
;;     v19 = load.i64 notrap aligned region5 v8
;;     v20 = load.i64 notrap aligned region6 v0+984
;;     v21 = load.i64 notrap aligned region5 v20
;;     v23 = load.i64 notrap aligned region4 v20+8
;;     v24 = uextend.i64 v4
;;     v26 = icmp ugt v24, v23
;;     v27 = isub v23, v24
;;     v28 = iconst.i64 1
;;     v29 = ushr v27, v28  ; v28 = 1
;;     v30 = icmp ugt v11, v29
;;     v31 = bor v26, v30
;;     trapnz v31, user33
;;     v33 = band v24, v28  ; v28 = 1
;;     trapnz v33, user36
;;     v39 = load.i64 notrap aligned readonly region7 v0+8
;;     v40 = load.i64 notrap aligned readonly can_move region8 v39+432
;;     v35 = iadd v19, v10
;;     v38 = iadd v21, v24
;;     v41 = call_indirect sig0, v40(v0, v35, v11, v38)
;;     v42 = iconst.i64 -1
;;     v43 = icmp ne v41, v42  ; v42 = -1
;;     brif v43, block2, block1
;;
;; block1 cold:
;;     v44 = load.i64 notrap aligned readonly can_move region9 v1+16
;;     v45 = load.i64 notrap aligned readonly can_move region10 v44+328
;;     call_indirect sig1, v45(v1)
;;     trap user1
;;
;; block2:
;;     v46 = ireduce.i32 v41
;;     return v46
;; }
;;
;; function u0:0(i64 vmctx, i64, i32, i32, i32, i32, i32) -> i32, i32 tail {
;;     ss0 = explicit_slot 8
;;     region0 = 123 ""
;;     region1 = 85 ""
;;     region2 = 72 ""
;;     region3 = 64 ""
;;     region4 = 105 ""
;;     region5 = 215 ""
;;     region6 = 50 ""
;;     region7 = 245 ""
;;     region8 = 155 ""
;;     region9 = 135 ""
;;     region10 = 12 ""
;;     region11 = 184 ""
;;     sig0 = (i64 uext, i64 uext, i64 uext, i64 uext, i64 uext, i32 uext, i64 uext) -> i64 uext system_v
;;     sig1 = (i64 uext vmctx) system_v
;;
;; block0(v0: i64, v1: i64, v2: i32, v3: i32, v4: i32, v5: i32, v6: i32):
;;     v8 = get_frame_pointer.i64 
;;     v7 = load.i64 notrap aligned readonly can_move region0 v1+8
;;     store notrap aligned region1 v8, v7+48
;;     v9 = get_return_address.i64 
;;     store notrap aligned region2 v9, v7+56
;;     v10 = load.i64 notrap aligned region3 v0+984
;;     v11 = load.i64 notrap aligned region4 v10+8
;;     v12 = uextend.i64 v2
;;     v14 = icmp ugt v12, v11
;;     v13 = uextend.i64 v3
;;     v15 = isub v11, v12
;;     v16 = iconst.i64 1
;;     v17 = ushr v15, v16  ; v16 = 1
;;     v18 = icmp ugt v13, v17
;;     v19 = bor v14, v18
;;     trapnz v19, user33
;;     v21 = band v12, v16  ; v16 = 1
;;     trapnz v21, user36
;;     v23 = load.i64 notrap aligned region5 v10
;;     v24 = load.i64 notrap aligned region6 v0+976
;;     v25 = load.i64 notrap aligned region5 v24
;;     v27 = load.i64 notrap aligned region4 v24+8
;;     v28 = uextend.i64 v4
;;     v30 = icmp ugt v28, v27
;;     v29 = uextend.i64 v5
;;     v31 = isub v27, v28
;;     v34 = icmp ugt v29, v31
;;     v35 = bor v30, v34
;;     trapnz v35, user33
;;     v43 = load.i64 notrap aligned readonly region7 v0+8
;;     v44 = load.i64 notrap aligned readonly can_move region8 v43+440
;;     v37 = iadd v23, v12
;;     v40 = iadd v25, v28
;;     v42 = stack_addr.i64 ss0
;;     v45 = call_indirect sig0, v44(v0, v37, v13, v40, v29, v6, v42)
;;     v46 = load.i64 notrap aligned region9 v42
;;     v47 = iconst.i64 -1
;;     v48 = icmp ne v45, v47  ; v47 = -1
;;     brif v48, block2, block1
;;
;; block1 cold:
;;     v49 = load.i64 notrap aligned readonly can_move region10 v1+16
;;     v50 = load.i64 notrap aligned readonly can_move region11 v49+328
;;     call_indirect sig1, v50(v1)
;;     trap user1
;;
;; block2:
;;     v51 = ireduce.i32 v45
;;     v52 = ireduce.i32 v46
;;     return v51, v52
;; }
