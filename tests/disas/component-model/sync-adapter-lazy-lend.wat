;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[2]--function"
;;! flags = "-Wconcurrency-support=y"

;; An adapter that passes a `borrow` to the instance owning the resource, with
;; both halves of the lazy-lend fast path inline and no libcall on either.

(component
  (component $A
    (type $t' (resource (rep i32)))
    (export $t "t" (type $t'))

    (core func $ctor (canon resource.new $t))

    (core module $M
      (func (export "get'") (param i32) (result i32)
        (local.get 0)
      )
    )
    (core instance $m (instantiate $M))

    (func (export "[constructor]t") (param "x" u32) (result (own $t))
      (canon lift (core func $ctor))
    )
    (func (export "get") (param "self" (borrow $t)) (result u32)
      (canon lift (core func $m "get'"))
    )
  )
  (instance $a (instantiate $A))

  (component $B
    (import "a" (instance $i
      (export "t" (type $t (sub resource)))
      (export "get" (func (param "self" (borrow $t)) (result u32)))
    ))
    (alias export $i "t" (type $t))
    (alias export $i "get" (func $get))

    (core func $get' (canon lower (func $get)))

    (core module $M
      (import "" "get" (func $get (param i32) (result i32)))
      (func (export "run") (param i32) (result i32)
        (call $get (local.get 0))
      )
    )
    (core instance $m (instantiate $M
      (with "" (instance (export "get" (func $get'))))))

    (func (export "run") (param "x" u32) (result u32)
      (canon lift (core func $m "run"))
    )
  )
  (instance $b (instantiate $B (with "a" (instance $a))))
  (export "run" (func $b "run"))
)
;; function u2:0(i64 vmctx, i64, i32) -> i32 tail {
;;     ss0 = explicit_slot 32, align = 8
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 57 ""
;;     region3 = 209 ""
;;     region4 = 42 ""
;;     region5 = 68 ""
;;     region6 = 237 ""
;;     region7 = 126 ""
;;     region8 = 221 ""
;;     region9 = 112 ""
;;     region10 = 71 ""
;;     region11 = 149 ""
;;     region12 = 93 ""
;;     region13 = 30 ""
;;     region14 = 184 ""
;;     region15 = 132 ""
;;     region16 = 108 ""
;;     region17 = 8 ""
;;     region18 = 218 ""
;;     region19 = 252 ""
;;     region20 = 13 ""
;;     region21 = 34 ""
;;     region22 = 195 ""
;;     region23 = 158 ""
;;     region24 = 174 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i64, i32, i32) tail
;;     sig2 = (i64 vmctx, i64, i32) -> i32 tail
;;     fn0 = colocated u0:0 sig2
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32):
;; @00e7                               jump block4
;;
;;                                 block6(v4: i64):
;; @00e7                               jump block3
;;
;;                                 block4:
;; @00ee                               v6 = load.i64 notrap aligned readonly can_move region2 v0+264
;; @00ee                               v7 = load.i32 notrap aligned region3 v6
;; @00f2                               trapz v7, user26
;; @00f2                               jump block7
;;
;;                                 block7:
;; @00fc                               v13 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @00fc                               v14 = load.i64 notrap aligned region5 v13+136
;; @00fc                               v12 = stack_addr.i64 ss0
;; @00fc                               store notrap aligned region6 v14, v12
;; @00e1                               v3 = iconst.i32 0
;; @00fc                               store notrap aligned region7 v3, v12+8  ; v3 = 0
;; @00fa                               v10 = iconst.i32 1
;; @00fc                               store notrap aligned region8 v10, v12+12  ; v10 = 1
;; @00fc                               v15 = load.i32 notrap aligned region9 v13+128
;; @00fc                               store notrap aligned region10 v15, v12+16
;; @00fc                               store notrap aligned region9 v3, v13+128  ; v3 = 0
;; @00fc                               v17 = load.i32 notrap aligned region11 v13+132
;; @00fc                               store notrap aligned region12 v17, v12+20
;; @00fc                               store notrap aligned region11 v3, v13+132  ; v3 = 0
;; @00fc                               v19 = load.i64 notrap aligned region13 v13+152
;; @00fc                               store notrap aligned region14 v19, v12+24
;; @00fc                               store notrap aligned region5 v12, v13+136
;; @00fe                               v20 = load.i64 notrap aligned readonly can_move region2 v0+240
;; @00fe                               v21 = load.i32 notrap aligned region15 v20
;; @0104                               store notrap aligned region15 v3, v20  ; v3 = 0
;; @0108                               jump block11
;;
;;                                 block11:
;; @0108                               v31 = iconst.i64 8
;; @0108                               v32 = icmp.i64 ult v19, v31  ; v31 = 8
;; @0108                               brif v32, block12, block9
;;
;;                                 block12:
;; @00f4                               v8 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @0108                               v36 = load.i64 notrap aligned region16 v8+120
;; @0108                               v33 = iconst.i32 -1
;; @0108                               v34 = iadd.i32 v2, v33  ; v33 = -1
;; @0108                               v35 = uextend.i64 v34
;; @0108                               v37 = icmp ult v35, v36
;; @0108                               brif v37, block13, block9
;;
;;                                 block13:
;; @0108                               v38 = load.i64 notrap aligned region17 v8+112
;; @0108                               v39 = iconst.i64 24
;; @0108                               v40 = imul.i64 v35, v39  ; v39 = 24
;; @0108                               v41 = iadd v38, v40
;; @0108                               v42 = load.i32 notrap aligned region18 v41
;; @0108                               v87 = iconst.i32 -1
;; @0108                               v88 = iadd v42, v87  ; v87 = -1
;; @00fa                               v89 = iconst.i32 1
;; @0108                               v90 = icmp ule v88, v89  ; v89 = 1
;; @0108                               brif v90, block14, block9
;;
;;                                 block14:
;; @0108                               v47 = load.i32 notrap aligned region19 v41+4
;; @0108                               v48 = iconst.i32 2
;; @0108                               v49 = icmp eq v47, v48  ; v48 = 2
;; @0108                               brif v49, block15, block9
;;
;;                                 block15:
;; @0108                               v50 = load.i32 notrap aligned region20 v41+8
;; @0108                               v91 = iconst.i32 2
;; @0108                               v51 = load.i64 notrap aligned readonly can_move region21 v13+144
;;                                     v84 = iconst.i64 3
;;                                     v85 = ishl.i64 v19, v84  ; v84 = 3
;; @0108                               v54 = iadd v51, v85
;; @0108                               store notrap aligned region22 v91, v54  ; v91 = 2
;; @0108                               store.i32 notrap aligned region23 v2, v54+4
;; @0108                               v56 = iconst.i64 1
;; @0108                               v57 = iadd.i64 v19, v56  ; v56 = 1
;; @0108                               store notrap aligned region13 v57, v13+152
;; @0108                               jump block10(v50)
;;
;;                                 block9 cold:
;; @0108                               v58 = load.i64 notrap aligned readonly can_move region24 v0+152
;; @00f4                               v92 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @0108                               try_call_indirect v58(v92, v0, v2), sig2, block16(ret0), [ context v0, default: block6(exn0) ]
;;
;;                                 block16(v59: i32):
;; @0108                               jump block10(v59)
;;
;;                                 block10(v27: i32):
;; @010c                               store.i32 notrap aligned region15 v21, v20
;; @010e                               v63 = load.i64 notrap aligned readonly can_move region4 v0+72
;; @010e                               try_call fn0(v63, v0, v27), sig2, block17(ret0), [ context v0, default: block6(exn0) ]
;;
;;                                 block17(v64: i32):
;; @0112                               v68 = load.i64 notrap aligned region5 v13+136
;; @0112                               v69 = icmp eq v68, v12
;; @0112                               brif v69, block18, block19
;;
;;                                 block18:
;; @0112                               v70 = load.i64 notrap aligned region6 v12
;; @0112                               store notrap aligned region5 v70, v13+136
;; @0112                               v71 = load.i64 notrap aligned region14 v12+24
;; @0112                               store notrap aligned region13 v71, v13+152
;; @0112                               v72 = load.i32 notrap aligned region10 v12+16
;; @0112                               store notrap aligned region9 v72, v13+128
;; @0112                               v73 = load.i32 notrap aligned region12 v12+20
;; @0112                               store notrap aligned region11 v73, v13+132
;; @0112                               jump block20
;;
;;                                 block19 cold:
;; @0112                               v74 = load.i64 notrap aligned readonly can_move region24 v0+184
;; @00f4                               v93 = load.i64 notrap aligned readonly can_move region4 v0+104
;; @0112                               try_call_indirect v74(v93, v0), sig0, block21, [ context v0, default: block6(exn0) ]
;;
;;                                 block21:
;; @0112                               jump block20
;;
;;                                 block20:
;; @011c                               store.i32 notrap aligned region3 v7, v6
;; @011e                               jump block5
;;
;;                                 block5:
;; @011f                               jump block2
;;
;;                                 block3:
;; @0122                               trap user52
;;
;;                                 block2:
;; @0126                               jump block1
;;
;;                                 block1:
;; @0126                               return v64
;; }
