;;! target = "x86_64"
;;! test = "compile"
;;! flags = [ "-Oopt-level=0", "-Cpcc=y", "-Ccranelift-has-sse41=true", "-Ccranelift-has-avx=false" ]

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
;;       movdqu  0x14(%rip), %xmm0
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       pinsrb  $1, (%rdi, %rsi), %xmm0
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
;;       movdqu  0x14(%rip), %xmm0
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       pinsrw  $1, (%rdi, %rsi), %xmm0
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
;;       movdqu  0x14(%rip), %xmm0
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       pinsrd  $1, (%rdi, %rsi), %xmm0
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
;;       movdqu  0x14(%rip), %xmm0
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       pinsrq  $1, (%rdi, %rsi), %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   df: addb    %ch, (%rax)
;;   e1: subl    %ebp, (%rcx)
;;   e3: subl    %ebp, (%rax)
;;   e5: imull   $0x616d286d, 0x20(%rsi), %ebp
;;
;; wasm[0]::function[4]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       insertps $0, (%rdi, %rsi), %xmm0
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
;;       movsd   (%r8, %rdi), %xmm7
;;       movlhps %xmm7, %xmm0
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
;;       movsd   (%r8, %rdi), %xmm7
;;       movsd   %xmm7, %xmm0
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[7]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pshufd  $0xee, %xmm0, %xmm6
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       movsd   %xmm6, (%rdi, %rsi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[8]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pshufd  $1, %xmm0, %xmm6
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       movss   %xmm6, (%rdi, %rsi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[9]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       pextrb  $1, %xmm0, %r8d
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
;;       pextrw  $1, %xmm0, %r8d
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
;;       pextrd  $1, %xmm0, (%rdi, %rsi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[12]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movl    %edx, %esi
;;       movq    0x38(%rdi), %rdi
;;       pextrq  $1, %xmm0, (%rdi, %rsi)
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
