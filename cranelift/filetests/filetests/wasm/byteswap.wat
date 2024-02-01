;;! target = "x86_64"
;;! optimize = true
;;! settings = ["opt_level=speed"]

(module
  (func (export "bswap32") (param i32) (result i32)
    local.get 0
    i32.const 24
    i32.shl
    local.get 0
    i32.const 65280
    i32.and
    i32.const 8
    i32.shl
    i32.or
    local.get 0
    i32.const 8
    i32.shr_u
    i32.const 65280
    i32.and
    local.get 0
    i32.const 24
    i32.shr_u
    i32.or
    i32.or
  )

  (func (export "bswap64") (param i64) (result i64)
    local.get 0
    i64.const 56
    i64.shl
    local.get 0
    i64.const 65280
    i64.and
    i64.const 40
    i64.shl
    i64.or
    local.get 0
    i64.const 16711680
    i64.and
    i64.const 24
    i64.shl
    local.get 0
    i64.const 4278190080
    i64.and
    i64.const 8
    i64.shl
    i64.or
    i64.or
    local.get 0
    i64.const 8
    i64.shr_u
    i64.const 4278190080
    i64.and
    local.get 0
    i64.const 24
    i64.shr_u
    i64.const 16711680
    i64.and
    i64.or
    local.get 0
    i64.const 40
    i64.shr_u
    i64.const 65280
    i64.and
    local.get 0
    i64.const 56
    i64.shr_u
    i64.or
    i64.or
    i64.or
  )
)

;; function u0:0(i32, i64 vmctx) -> i32 fast {
;;                                 block0(v0: i32, v1: i64):
;; @0057                               jump block1
;;
;;                                 block1:
;;                                     v18 = bswap.i32 v0
;; @0057                               return v18
;; }
;;
;; function u0:1(i64, i64 vmctx) -> i64 fast {
;;                                 block0(v0: i64, v1: i64):
;; @00ad                               jump block1
;;
;;                                 block1:
;;                                     v38 = bswap.i64 v0
;; @00ad                               return v38
;; }
