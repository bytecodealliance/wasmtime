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
;;       vmovdqu 0x14(%rip), %xmm6
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       vpinsrb $1, (%rdi, %rsi), %xmm6, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   1e: addb    %al, (%rax)
;;   20: subb    %ch, (%rcx)
;;   22: subl    %ebp, (%rcx)
;;   24: subb    %ch, 0x6e(%rcx)
;;   27: andb    %ch, 0x28(%rbp)
;;   2a: insl    %dx, (%rdi)
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vmovdqu 0x14(%rip), %xmm6
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       vpinsrw $1, (%rdi, %rsi), %xmm6, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   5d: addb    %al, (%rax)
;;   5f: addb    %ch, (%rax)
;;   61: subl    %ebp, (%rcx)
;;   63: subl    %ebp, (%rax)
;;   65: imull   $0x616d286d, 0x20(%rsi), %ebp
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vmovdqu 0x14(%rip), %xmm6
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       vpinsrd $1, (%rdi, %rsi), %xmm6, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   9e: addb    %al, (%rax)
;;   a0: subb    %ch, (%rcx)
;;   a2: subl    %ebp, (%rcx)
;;   a4: subb    %ch, 0x6e(%rcx)
;;   a7: andb    %ch, 0x28(%rbp)
;;   aa: insl    %dx, (%rdi)
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vmovdqu 0x14(%rip), %xmm6
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       vpinsrq $1, (%rdi, %rsi), %xmm6, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   de: addb    %al, (%rax)
;;   e0: subb    %ch, (%rcx)
;;   e2: subl    %ebp, (%rcx)
;;   e4: subb    %ch, 0x6e(%rcx)
;;   e7: andb    %ch, 0x28(%rbp)
;;   ea: insl    %dx, (%rdi)
;;
;; wasm[0]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       vinsertps $0, (%rdi, %rsi), %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[5]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rdi, %r9
;;       movl    %edx, %edi
;;       movq    0x38(%r9), %r8
;;       vmovsd  (%r8, %rdi), %xmm7
;;       vmovlhps %xmm7, %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[6]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rdi, %r9
;;       movl    %edx, %edi
;;       movq    0x38(%r9), %r8
;;       vmovsd  (%r8, %rdi), %xmm7
;;       vmovsd  %xmm7, %xmm0, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[7]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpshufd $0xee, %xmm0, %xmm6
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       vmovsd  %xmm6, (%rdi, %rsi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[8]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpshufd $1, %xmm0, %xmm6
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       vmovss  %xmm6, (%rdi, %rsi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[9]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpextrb $1, %xmm0, %r8d
;;       movsbl  %r8b, %r8d
;;       movl    %edx, %r9d
;;       movq    0x38(%rdi), %rdi
;;       movb    %r8b, (%rdi, %r9)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[10]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       vpextrw $1, %xmm0, %r8d
;;       movswl  %r8w, %r8d
;;       movl    %edx, %r9d
;;       movq    0x38(%rdi), %rdi
;;       movw    %r8w, (%rdi, %r9)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[11]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       vpextrd $1, %xmm0, (%rdi, %rsi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[12]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       vpextrq $1, %xmm0, (%rdi, %rsi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
