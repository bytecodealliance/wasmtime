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
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vmovdqu 0x14(%rip), %xmm7
;;       movl    %edx, %r10d
;;       movq    0x60(%rdi), %r11
;;       vpinsrb $1, (%r11, %r10), %xmm7, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   1f: addb    %ch, (%rax)
;;   21: subl    %ebp, (%rcx)
;;   23: subl    %ebp, (%rax)
;;   25: imull   $0x616d286d, 0x20(%rsi), %ebp
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vmovdqu 0x14(%rip), %xmm7
;;       movl    %edx, %r10d
;;       movq    0x60(%rdi), %r11
;;       vpinsrw $1, (%r11, %r10), %xmm7, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   4f: addb    %ch, (%rax)
;;   51: subl    %ebp, (%rcx)
;;   53: subl    %ebp, (%rax)
;;   55: imull   $0x616d286d, 0x20(%rsi), %ebp
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vmovdqu 0x14(%rip), %xmm7
;;       movl    %edx, %r10d
;;       movq    0x60(%rdi), %r11
;;       vpinsrd $1, (%r11, %r10), %xmm7, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   7f: addb    %ch, (%rax)
;;   81: subl    %ebp, (%rcx)
;;   83: subl    %ebp, (%rax)
;;   85: imull   $0x616d286d, 0x20(%rsi), %ebp
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vmovdqu 0x14(%rip), %xmm7
;;       movl    %edx, %r10d
;;       movq    0x60(%rdi), %r11
;;       vpinsrq $1, (%r11, %r10), %xmm7, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   af: addb    %ch, (%rax)
;;   b1: subl    %ebp, (%rcx)
;;   b3: subl    %ebp, (%rax)
;;   b5: imull   $0x616d286d, 0x20(%rsi), %ebp
;;
;; wasm[0]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %r10d
;;       movq    0x60(%rdi), %r11
;;       vinsertps $0, (%r11, %r10), %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %r10d
;;       movq    0x60(%rdi), %r11
;;       vmovhps (%r11, %r10), %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[6]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %r11d
;;       movq    0x60(%rdi), %rsi
;;       vmovsd  (%rsi, %r11), %xmm1
;;       vmovsd  %xmm1, %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[7]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpshufd $0xee, %xmm0, %xmm7
;;       movl    %edx, %r10d
;;       movq    0x60(%rdi), %r11
;;       vmovsd  %xmm7, (%r11, %r10)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[8]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpshufd $1, %xmm0, %xmm7
;;       movl    %edx, %r10d
;;       movq    0x60(%rdi), %r11
;;       vmovss  %xmm7, (%r11, %r10)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[9]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpextrb $1, %xmm0, %r11d
;;       movsbl  %r11b, %r11d
;;       movl    %edx, %esi
;;       movq    0x60(%rdi), %rdi
;;       movb    %r11b, (%rdi, %rsi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[10]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpextrw $1, %xmm0, %r11d
;;       movswl  %r11w, %r11d
;;       movl    %edx, %esi
;;       movq    0x60(%rdi), %rdi
;;       movw    %r11w, (%rdi, %rsi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[11]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %r9d
;;       movq    0x60(%rdi), %r10
;;       vpextrd $1, %xmm0, (%r10, %r9)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[12]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %r9d
;;       movq    0x60(%rdi), %r10
;;       vpextrq $1, %xmm0, (%r10, %r9)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
