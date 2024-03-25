;;! target = "x86_64"
;;! test = "compile"
;;! flags = [ "-Oopt-level=0", "-Cpcc=y", "-Ccranelift-has-sse41=true", "-Ccranelift-has-avx=true" ]

(module
  (memory 1 1)
  (func (param i32) (result v128)
      local.get 0
      v128.const i32x4 0x29292928 0x206e6928 0x616d286d 0x206f7263
      v128.load8_lane align=1 1)
  (func (param i32) (result v128)
      local.get 0
      v128.const i32x4 0x29292928 0x206e6928 0x616d286d 0x206f7263
      v128.load16_lane align=1 1)
  (func (param i32) (result v128)
      local.get 0
      v128.const i32x4 0x29292928 0x206e6928 0x616d286d 0x206f7263
      v128.load32_lane align=1 1)
  (func (param i32) (result v128)
      local.get 0
      v128.const i32x4 0x29292928 0x206e6928 0x616d286d 0x206f7263
      v128.load64_lane align=1 1)
  (func (param v128 i32) (result v128)
		local.get 0
		local.get 1
		f32.load
		f32x4.replace_lane 0)
  (func (param v128 i32) (result v128)
		local.get 0
		local.get 1
		f64.load
		f64x2.replace_lane 1)
  (func (param v128 i32) (result v128)
		local.get 0
		local.get 1
		f64.load
		f64x2.replace_lane 0)
  (func (param v128 i32)
		local.get 1
		local.get 0
		f64x2.extract_lane 1
		f64.store)
  (func (param v128 i32)
		local.get 1
		local.get 0
		f32x4.extract_lane 1
		f32.store)
  (func (param v128 i32)
		local.get 1
		local.get 0
		i8x16.extract_lane_s 1
		i32.store8)
  (func (param v128 i32)
		local.get 1
		local.get 0
		i16x8.extract_lane_s 1
		i32.store16)
  (func (param v128 i32)
		local.get 1
		local.get 0
		i32x4.extract_lane 1
		i32.store)
  (func (param v128 i32)
		local.get 1
		local.get 0
		i64x2.extract_lane 1
		i64.store))
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: vmovdqu 0x14(%rip), %xmm7
;;    c: movl    %edx, %r10d
;;    f: movq    0x50(%rdi), %r11
;;   13: vpinsrb $1, (%r11, %r10), %xmm7, %xmm0
;;   1a: movq    %rbp, %rsp
;;   1d: popq    %rbp
;;   1e: retq
;;   1f: addb    %ch, (%rax)
;;   21: subl    %ebp, (%rcx)
;;   23: subl    %ebp, (%rax)
;;   25: imull   $0x616d286d, 0x20(%rsi), %ebp
;;
;; wasm[0]::function[1]:
;;   30: pushq   %rbp
;;   31: movq    %rsp, %rbp
;;   34: vmovdqu 0x14(%rip), %xmm7
;;   3c: movl    %edx, %r10d
;;   3f: movq    0x50(%rdi), %r11
;;   43: vpinsrw $1, (%r11, %r10), %xmm7, %xmm0
;;   4a: movq    %rbp, %rsp
;;   4d: popq    %rbp
;;   4e: retq
;;   4f: addb    %ch, (%rax)
;;   51: subl    %ebp, (%rcx)
;;   53: subl    %ebp, (%rax)
;;   55: imull   $0x616d286d, 0x20(%rsi), %ebp
;;
;; wasm[0]::function[2]:
;;   60: pushq   %rbp
;;   61: movq    %rsp, %rbp
;;   64: vmovdqu 0x14(%rip), %xmm7
;;   6c: movl    %edx, %r10d
;;   6f: movq    0x50(%rdi), %r11
;;   73: vpinsrd $1, (%r11, %r10), %xmm7, %xmm0
;;   7a: movq    %rbp, %rsp
;;   7d: popq    %rbp
;;   7e: retq
;;   7f: addb    %ch, (%rax)
;;   81: subl    %ebp, (%rcx)
;;   83: subl    %ebp, (%rax)
;;   85: imull   $0x616d286d, 0x20(%rsi), %ebp
;;
;; wasm[0]::function[3]:
;;   90: pushq   %rbp
;;   91: movq    %rsp, %rbp
;;   94: vmovdqu 0x14(%rip), %xmm7
;;   9c: movl    %edx, %r10d
;;   9f: movq    0x50(%rdi), %r11
;;   a3: vpinsrq $1, (%r11, %r10), %xmm7, %xmm0
;;   aa: movq    %rbp, %rsp
;;   ad: popq    %rbp
;;   ae: retq
;;   af: addb    %ch, (%rax)
;;   b1: subl    %ebp, (%rcx)
;;   b3: subl    %ebp, (%rax)
;;   b5: imull   $0x616d286d, 0x20(%rsi), %ebp
;;
;; wasm[0]::function[4]:
;;   c0: pushq   %rbp
;;   c1: movq    %rsp, %rbp
;;   c4: movl    %edx, %r10d
;;   c7: movq    0x50(%rdi), %r11
;;   cb: vinsertps $0, (%r11, %r10), %xmm0, %xmm0
;;   d2: movq    %rbp, %rsp
;;   d5: popq    %rbp
;;   d6: retq
;;
;; wasm[0]::function[5]:
;;   e0: pushq   %rbp
;;   e1: movq    %rsp, %rbp
;;   e4: movl    %edx, %r10d
;;   e7: movq    0x50(%rdi), %r11
;;   eb: vmovhps (%r11, %r10), %xmm0, %xmm0
;;   f1: movq    %rbp, %rsp
;;   f4: popq    %rbp
;;   f5: retq
;;
;; wasm[0]::function[6]:
;;  100: pushq   %rbp
;;  101: movq    %rsp, %rbp
;;  104: movl    %edx, %r11d
;;  107: movq    0x50(%rdi), %rsi
;;  10b: vmovsd  (%rsi, %r11), %xmm1
;;  111: vmovsd  %xmm1, %xmm0, %xmm0
;;  115: movq    %rbp, %rsp
;;  118: popq    %rbp
;;  119: retq
;;
;; wasm[0]::function[7]:
;;  120: pushq   %rbp
;;  121: movq    %rsp, %rbp
;;  124: vpshufd $0xee, %xmm0, %xmm7
;;  129: movl    %edx, %r10d
;;  12c: movq    0x50(%rdi), %r11
;;  130: vmovsd  %xmm7, (%r11, %r10)
;;  136: movq    %rbp, %rsp
;;  139: popq    %rbp
;;  13a: retq
;;
;; wasm[0]::function[8]:
;;  140: pushq   %rbp
;;  141: movq    %rsp, %rbp
;;  144: vpshufd $1, %xmm0, %xmm7
;;  149: movl    %edx, %r10d
;;  14c: movq    0x50(%rdi), %r11
;;  150: vmovss  %xmm7, (%r11, %r10)
;;  156: movq    %rbp, %rsp
;;  159: popq    %rbp
;;  15a: retq
;;
;; wasm[0]::function[9]:
;;  160: pushq   %rbp
;;  161: movq    %rsp, %rbp
;;  164: vpextrb $1, %xmm0, %r11d
;;  16a: movsbl  %r11b, %r11d
;;  16e: movl    %edx, %esi
;;  170: movq    0x50(%rdi), %rdi
;;  174: movb    %r11b, (%rdi, %rsi)
;;  178: movq    %rbp, %rsp
;;  17b: popq    %rbp
;;  17c: retq
;;
;; wasm[0]::function[10]:
;;  180: pushq   %rbp
;;  181: movq    %rsp, %rbp
;;  184: vpextrw $1, %xmm0, %r11d
;;  18a: movswl  %r11w, %r11d
;;  18e: movl    %edx, %esi
;;  190: movq    0x50(%rdi), %rdi
;;  194: movw    %r11w, (%rdi, %rsi)
;;  199: movq    %rbp, %rsp
;;  19c: popq    %rbp
;;  19d: retq
;;
;; wasm[0]::function[11]:
;;  1a0: pushq   %rbp
;;  1a1: movq    %rsp, %rbp
;;  1a4: movl    %edx, %r9d
;;  1a7: movq    0x50(%rdi), %r10
;;  1ab: vpextrd $1, %xmm0, (%r10, %r9)
;;  1b2: movq    %rbp, %rsp
;;  1b5: popq    %rbp
;;  1b6: retq
;;
;; wasm[0]::function[12]:
;;  1c0: pushq   %rbp
;;  1c1: movq    %rsp, %rbp
;;  1c4: movl    %edx, %r9d
;;  1c7: movq    0x50(%rdi), %r10
;;  1cb: vpextrq $1, %xmm0, (%r10, %r9)
;;  1d2: movq    %rbp, %rsp
;;  1d5: popq    %rbp
;;  1d6: retq
