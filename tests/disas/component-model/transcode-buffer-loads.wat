;;! target = "x86_64"
;;! test = "optimize"
;;! filter = "wasm[3]--function"

;; Before calling a host string transcoder the adapter loads the last code unit
;; of each buffer the host will access, so an out-of-bounds buffer traps in wasm.

(component
  (component $callee
    (core module $m
      (memory (export "memory") 1)
      (func (export "realloc") (param i32 i32 i32 i32) (result i32) unreachable)
      (func (export "f") (param i32 i32) unreachable)
    )
    (core instance $m (instantiate $m))
    (func (export "f") (param "a" string)
      (canon lift (core func $m "f")
        (memory (core memory $m "memory"))
        (realloc (core func $m "realloc"))
        string-encoding=utf16))
  )

  (component $caller
    (import "f" (func $f (param "a" string)))
    (core module $libc
      (memory (export "memory") 1)
    )
    (core instance $libc (instantiate $libc))
    (core func $f
      (canon lower (func $f)
        (memory (core memory $libc "memory"))
        string-encoding=utf8))
    (core module $m
      (import "" "f" (func (param i32 i32)))
    )
    (core instance (instantiate $m (with "" (instance (export "f" (func $f))))))
  )

  (instance $callee (instantiate $callee))
  (instance (instantiate $caller (with "f" (func $callee "f"))))
)
;; function u3:0(i64 vmctx, i64, i32, i32) tail {
;;     region0 = 123 ""
;;     region1 = 160 ""
;;     region2 = 44 ""
;;     region3 = 78 ""
;;     region4 = 25 ""
;;     region5 = 227 ""
;;     region6 = 13 ""
;;     region7 = 215 ""
;;     region8 = 105 ""
;;     region9 = 112 ""
;;     region10 = 149 ""
;;     region11 = 184 ""
;;     region12 = 171 ""
;;     region13 = 68 ""
;;     gv0 = vmctx
;;     gv1 = load.i64 notrap aligned readonly can_move region0 gv0+8
;;     gv2 = load.i64 notrap aligned region1 gv1+24
;;     sig0 = (i64 vmctx, i64) tail
;;     sig1 = (i64 vmctx, i64) -> i32 tail
;;     sig2 = (i64 vmctx, i64, i32) tail
;;     sig3 = (i64 vmctx, i64, i32, i32, i32, i32) -> i32 tail
;;     sig4 = (i64 vmctx, i64, i32, i32, i32) -> i32 tail
;;     sig5 = (i64 vmctx, i64, i32, i32) tail
;;     fn0 = colocated u2147483648:17 sig1
;;     fn1 = colocated u2147483648:19 sig1
;;     fn2 = colocated u2147483648:18 sig2
;;     fn3 = colocated u2147483648:20 sig2
;;     fn4 = colocated u0:0 sig3
;;     fn5 = colocated u0:1 sig5
;;     stack_limit = gv2
;;
;;                                 block0(v0: i64, v1: i64, v2: i32, v3: i32):
;; @0155                               jump block4
;;
;;                                 block6(v5: i64):
;; @0155                               jump block3
;;
;;                                 block4:
;; @015c                               v7 = load.i64 notrap aligned readonly can_move region2 v0+472
;; @015c                               v8 = load.i32 notrap aligned region3 v7
;; @0160                               trapz v8, user26
;; @0160                               jump block7
;;
;;                                 block7:
;; @0166                               v10 = load.i64 notrap aligned readonly can_move region2 v0+448
;; @0166                               v11 = load.i32 notrap aligned region5 v10
;; @014f                               v4 = iconst.i32 0
;; @016c                               store notrap aligned region5 v4, v10  ; v4 = 0
;; @017a                               v14 = load.i64 notrap aligned readonly can_move region6 v0+72
;; @017a                               v15 = load.i64 notrap aligned region8 v14+8
;; @017a                               v16 = iconst.i64 16
;; @017a                               v17 = ushr v15, v16  ; v16 = 16
;; @017a                               v18 = ireduce.i32 v17
;; @017c                               v19 = uextend.i64 v18
;; @017f                               v21 = ishl v19, v16  ; v16 = 16
;; @0182                               v22 = uextend.i64 v2
;; @0185                               v23 = uextend.i64 v3
;; @0186                               v24 = iadd v22, v23
;; @0187                               v25 = icmp uge v21, v24
;; @0188                               brif v25, block9, block11
;;
;;                                 block11:
;; @018a                               jump block10
;;
;;                                 block10:
;; @018b                               trap user33
;;
;;                                 block9:
;; @0191                               v28 = iconst.i32 0x3fff_ffff
;; @0197                               v29 = icmp.i32 ugt v3, v28  ; v28 = 0x3fff_ffff
;; @0198                               trapnz v29, user33
;; @0198                               jump block13
;;
;;                                 block13:
;; @01af                               v38 = load.i64 notrap aligned readonly can_move region0 v0+8
;; @01af                               v39 = load.i32 notrap aligned region9 v38+128
;; @01b3                               v42 = load.i32 notrap aligned region10 v38+132
;;                                     v192 = iconst.i32 0
;; @01b9                               store notrap aligned region9 v192, v38+128  ; v192 = 0
;; @01bd                               store notrap aligned region10 v192, v38+132  ; v192 = 0
;; @01bf                               v49 = load.i64 notrap aligned readonly can_move region4 v0+120
;; @01ab                               v36 = iconst.i32 2
;; @01a2                               v32 = iconst.i32 1
;; @01a4                               v33 = ishl.i32 v3, v32  ; v32 = 1
;; @01bf                               try_call fn4(v49, v0, v192, v192, v36, v33), sig3, block14(ret0), [ context v0, default: block6(exn0) ]  ; v192 = 0, v192 = 0, v36 = 2
;;
;;                                 block14(v50: i32):
;; @01c3                               store.i32 notrap aligned region9 v39, v38+128
;; @01c7                               store.i32 notrap aligned region10 v42, v38+132
;;                                     v193 = iconst.i32 1
;;                                     v194 = band v50, v193  ; v193 = 1
;; @01d0                               trapnz v194, user36
;; @01d0                               jump block16
;;
;;                                 block16:
;; @01da                               v58 = load.i64 notrap aligned readonly can_move region6 v0+48
;; @01da                               v59 = load.i64 notrap aligned region8 v58+8
;;                                     v195 = iconst.i64 16
;;                                     v196 = ushr v59, v195  ; v195 = 16
;; @01da                               v62 = ireduce.i32 v196
;; @01dc                               v63 = uextend.i64 v62
;;                                     v197 = ishl v63, v195  ; v195 = 16
;; @01e2                               v66 = uextend.i64 v50
;; @01e5                               v67 = uextend.i64 v33
;; @01e6                               v68 = iadd v66, v67
;; @01e7                               v69 = icmp uge v197, v68
;; @01e8                               brif v69, block17, block19
;;
;;                                 block19:
;; @01ea                               jump block18
;;
;;                                 block18:
;; @01eb                               trap user33
;;
;;                                 block17:
;; @01f4                               v73 = iconst.i64 0
;; @01f4                               v74 = icmp.i64 eq v23, v73  ; v73 = 0
;; @01f5                               brif v74, block20, block21
;;
;;                                 block21:
;; @01fa                               v77 = iconst.i64 0x7fff_ffff
;; @0200                               v78 = icmp.i64 ugt v23, v77  ; v77 = 0x7fff_ffff
;; @0201                               trapnz v78, user33
;; @0201                               jump block23
;;
;;                                 block23:
;;                                     v198 = iconst.i32 1
;;                                     v199 = isub.i32 v3, v198  ; v198 = 1
;; @0210                               v85 = iadd.i32 v2, v199
;; @0215                               v86 = icmp ult v85, v2
;; @0216                               trapnz v86, user33
;; @0216                               jump block25
;;
;;                                 block25:
;; @021e                               v91 = load.i64 notrap aligned readonly can_move region7 v14
;; @021e                               v89 = uextend.i64 v85
;; @021e                               v92 = iadd v91, v89
;; @021e                               v93 = uload8.i32 little region11 v92
;; @0223                               jump block20
;;
;;                                 block20:
;; @0229                               jump block27
;;
;;                                 block27:
;; @0235                               brif.i8 v74, block28, block29
;;
;;                                 block29:
;; @023a                               v104 = iconst.i64 0x3fff_ffff
;; @0240                               v105 = icmp.i64 ugt v23, v104  ; v104 = 0x3fff_ffff
;; @0241                               trapnz v105, user33
;; @0241                               jump block31
;;
;;                                 block31:
;; @020c                               v82 = iconst.i64 1
;; @020e                               v83 = isub.i64 v23, v82  ; v82 = 1
;; @0251                               v112 = ishl v83, v82  ; v82 = 1
;; @0252                               v113 = ireduce.i32 v112
;; @0253                               v114 = iadd.i32 v50, v113
;; @0258                               v115 = icmp ult v114, v50
;; @0259                               trapnz v115, user33
;; @0259                               jump block33
;;
;;                                 block33:
;; @0261                               v120 = load.i64 notrap aligned readonly can_move region7 v58
;; @0261                               v118 = uextend.i64 v114
;; @0261                               v121 = iadd v120, v118
;; @0261                               v122 = uload16.i32 little region12 v121
;; @0265                               jump block28
;;
;;                                 block28:
;; @026c                               v128 = load.i64 notrap aligned readonly can_move region13 v0+392
;; @0162                               v9 = load.i64 notrap aligned readonly can_move region4 v0+184
;; @026c                               try_call_indirect v128(v9, v0, v2, v3, v50), sig4, block34(ret0), [ context v0, default: block6(exn0) ]
;;
;;                                 block34(v129: i32):
;; @0274                               v130 = icmp.i32 ne v3, v129
;; @0275                               brif v130, block35, block36(v50)
;;
;;                                 block35:
;; @0282                               v139 = load.i32 notrap aligned region9 v38+128
;; @0286                               v142 = load.i32 notrap aligned region10 v38+132
;;                                     v200 = iconst.i32 0
;; @028c                               store notrap aligned region9 v200, v38+128  ; v200 = 0
;; @0290                               store notrap aligned region10 v200, v38+132  ; v200 = 0
;;                                     v201 = iconst.i32 2
;;                                     v202 = iconst.i32 1
;;                                     v203 = ishl.i32 v129, v202  ; v202 = 1
;; @0292                               try_call fn4(v49, v0, v50, v33, v201, v203), sig3, block37(ret0), [ context v0, default: block6(exn0) ]  ; v201 = 2
;;
;;                                 block37(v150: i32):
;; @0296                               store.i32 notrap aligned region9 v139, v38+128
;; @029a                               store.i32 notrap aligned region10 v142, v38+132
;;                                     v204 = iconst.i32 1
;;                                     v205 = band v150, v204  ; v204 = 1
;; @02a3                               trapnz v205, user36
;; @02a3                               jump block39
;;
;;                                 block39:
;; @02ad                               v159 = load.i64 notrap aligned region8 v58+8
;;                                     v206 = iconst.i64 16
;;                                     v207 = ushr v159, v206  ; v206 = 16
;; @02ad                               v162 = ireduce.i32 v207
;; @02af                               v163 = uextend.i64 v162
;;                                     v208 = ishl v163, v206  ; v206 = 16
;; @02b5                               v166 = uextend.i64 v150
;; @02bb                               v169 = uextend.i64 v203
;; @02bc                               v170 = iadd v166, v169
;; @02bd                               v171 = icmp uge v208, v170
;; @02be                               brif v171, block40, block42
;;
;;                                 block42:
;; @02c0                               jump block41
;;
;;                                 block41:
;; @02c1                               trap user33
;;
;;                                 block40:
;; @02c5                               jump block36(v150)
;;
;;                                 block36(v174: i32):
;; @02cc                               store.i32 notrap aligned region5 v11, v10
;; @02ce                               try_call fn5(v49, v0, v174, v129), sig5, block43, [ context v0, default: block6(exn0) ]
;;
;;                                 block43:
;; @02d6                               store.i32 notrap aligned region3 v8, v7
;; @02d8                               jump block5
;;
;;                                 block5:
;; @02d9                               jump block2
;;
;;                                 block3:
;; @02dc                               trap user52
;;
;;                                 block2:
;; @02e0                               jump block1
;;
;;                                 block1:
;; @02e0                               return
;; }
