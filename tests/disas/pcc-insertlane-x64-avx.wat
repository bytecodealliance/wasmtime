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
;; function u0:0:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vmovdqu const(0), %xmm7
;;   movl    %edx, %r10d
;;   movq    80(%rdi), %r11
;;   vpinsrb $1, %xmm7, 0(%r11,%r10,1), %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:1:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vmovdqu const(0), %xmm7
;;   movl    %edx, %r10d
;;   movq    80(%rdi), %r11
;;   vpinsrw $1, %xmm7, 0(%r11,%r10,1), %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:2:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vmovdqu const(0), %xmm7
;;   movl    %edx, %r10d
;;   movq    80(%rdi), %r11
;;   vpinsrd $1, %xmm7, 0(%r11,%r10,1), %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:3:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vmovdqu const(0), %xmm7
;;   movl    %edx, %r10d
;;   movq    80(%rdi), %r11
;;   vpinsrq $1, %xmm7, 0(%r11,%r10,1), %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:4:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   movl    %edx, %r10d
;;   movq    80(%rdi), %r11
;;   vinsertps $0, %xmm0, 0(%r11,%r10,1), %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:5:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   movl    %edx, %r10d
;;   movq    80(%rdi), %r11
;;   vmovlhps %xmm0, 0(%r11,%r10,1), %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:6:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   movl    %edx, %r11d
;;   movq    80(%rdi), %rsi
;;   vmovsd  0(%rsi,%r11,1), %xmm1
;;   vmovsd  %xmm0, %xmm1, %xmm0
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:7:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vpshufd $238, %xmm0, %xmm7
;;   movl    %edx, %r10d
;;   movq    80(%rdi), %r11
;;   vmovsd  %xmm7, 0(%r11,%r10,1)
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:8:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vpshufd $1, %xmm0, %xmm7
;;   movl    %edx, %r10d
;;   movq    80(%rdi), %r11
;;   vmovss  %xmm7, 0(%r11,%r10,1)
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:9:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vpextrb $1, %xmm0, %r11
;;   movsbl  %r11b, %r11d
;;   movl    %edx, %esi
;;   movq    80(%rdi), %rdi
;;   movb    %r11b, 0(%rdi,%rsi,1)
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:10:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   vpextrw $1, %xmm0, %r11
;;   movswl  %r11w, %r11d
;;   movl    %edx, %esi
;;   movq    80(%rdi), %rdi
;;   movw    %r11w, 0(%rdi,%rsi,1)
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:11:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   movl    %edx, %r9d
;;   movq    80(%rdi), %r10
;;   vpextrd $1, %xmm0, 0(%r10,%r9,1)
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:12:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   movl    %edx, %r9d
;;   movq    80(%rdi), %r10
;;   vpextrq $1, %xmm0, 0(%r10,%r9,1)
;;   jmp     label1
;; block1:
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
