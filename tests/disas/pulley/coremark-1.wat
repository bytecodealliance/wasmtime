;;! target = "pulley64"
;;! test = "compile"

;; Small test of a loop extracted from "coremark-minimal.wasm"  here:
;; https://github.com/wasmi-labs/wasmi-benchmarks/blob/d045a88246d3ac9b0b80b188feda54b89ca126b5/benches/res/wasm/coremark-minimal.wasm
;;
;; This doesn't reproduce the exact regalloc decisions but does currently show
;; something suboptimal for Pulley which is at the end of the loop it's
;; currently:
;;
;; * `br_if_not32` to exit the loop
;; * `xmov` to move some registers in place
;; * `jump` to resume the loop
;;
;; Ideally to minimize Pulley opcodes this would skip the `xmov` and `jump`
;; with different register allocation and the back-edge would be a single
;; conditional branch.

(module
  (memory 10)
  (func (param $p1 i32) (param $p2 i32) (param $cnt i32)
        (param $stride i32)
        (result i32)
    (local $accum i32)
    loop
      local.get $accum

      local.get $p1
      i32.load16_u
      local.get $p2
      i32.load16_u
      i32.mul
      local.tee $accum
      i32.const 2
      i32.shr_u
      i32.const 15
      i32.and
      local.get $accum
      i32.const 5
      i32.shr_u
      i32.const 127
      i32.and
      i32.mul
      i32.add
      local.set $accum

      local.get $p2
      i32.const 2
      i32.add
      local.set $p2

      local.get $p1
      local.get $stride
      i32.add
      local.set $p1

      local.get $cnt
      i32.const -1
      i32.add
      local.tee $cnt

      br_if 0
    end

    call $other

    (local.get $accum)
  )

  (func $other)
)
;; wasm[0]::function[0]:
;;       push_frame_save 16, x16
;;       xzero x6
;;       xload64le_o32 x11, x0, 80
;;       xload64le_o32 x13, x0, 88
;;       xload16le_u32_g32 x12, x11, x13, x2, 0
;;       xload16le_u32_g32 x13, x11, x13, x3, 0
;;       xsub32_u8 x4, x4, 1
;;       xmul32 x12, x12, x13
;;       xshr32_u_u6 x13, x12, 2
;;       xband32_s8 x13, x13, 15
;;       xshr32_u_u6 x12, x12, 5
;;       xband32_s8 x12, x12, 127
;;       xmadd32 x6, x13, x12, x6
;;       xmov x16, x6
;;       xadd32 x2, x2, x5
;;       xadd32_u8 x3, x3, 2
;;       br_if_not32 x4, 0xe    // target = 0x53
;;   4b: xmov x6, x16
;;       jump -0x40    // target = 0xe
;;   53: call2 x0, x0, 0x10    // target = 0x63
;;       xmov x0, x16
;;       pop_frame_restore 16, x16
;;       ret
;;
;; wasm[0]::function[1]::other:
;;       push_frame
;;       pop_frame
;;       ret
