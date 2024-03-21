;;! target = "x86_64"
(module
  (type (;0;) (func (param f32 f64 i64 i64 f64 f64 f32) (result f32 f64 f32)))
  (func (;0;) (type 0) (param f32 f64 i64 i64 f64 f64 f32) (result f32 f64 f32)
    global.get 1
    i32.eqz
    if ;; label = @1
      unreachable
    end
    global.get 1
    i32.const 1
    i32.sub
    global.set 1
    memory.size
    memory.grow
    i64.load8_s offset=143396
    (drop)
    (f32.const 0)
    (f64.const 0)
    (f32.const 0)
  )
  (memory (;1;) 10 10)
  (global (;0;) f32 f32.const 0x1.d6a0d6p+87 (;=284477330000000000000000000;))
  (global (;1;) (mut i32) i32.const 1000)
  (export "main" (func 0))
  (export "0" (memory 0))
  (export "1" (global 0))
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c360000000       	add	r11, 0x60
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f87f1000000         	ja	0x10c
;;   1b:	 4989fe               	mov	r14, rdi
;;      	 4883ec50             	sub	rsp, 0x50
;;      	 48897c2448           	mov	qword ptr [rsp + 0x48], rdi
;;      	 4889742440           	mov	qword ptr [rsp + 0x40], rsi
;;      	 f30f1144243c         	movss	dword ptr [rsp + 0x3c], xmm0
;;      	 f20f114c2430         	movsd	qword ptr [rsp + 0x30], xmm1
;;      	 4889542428           	mov	qword ptr [rsp + 0x28], rdx
;;      	 48894c2420           	mov	qword ptr [rsp + 0x20], rcx
;;      	 f20f11542418         	movsd	qword ptr [rsp + 0x18], xmm2
;;      	 f20f115c2410         	movsd	qword ptr [rsp + 0x10], xmm3
;;      	 f30f1164240c         	movss	dword ptr [rsp + 0xc], xmm4
;;      	 4c890424             	mov	qword ptr [rsp], r8
;;      	 418b4670             	mov	eax, dword ptr [r14 + 0x70]
;;      	 83f800               	cmp	eax, 0
;;      	 b800000000           	mov	eax, 0
;;      	 400f94c0             	sete	al
;;      	 85c0                 	test	eax, eax
;;      	 0f8402000000         	je	0x72
;;   70:	 0f0b                 	ud2	
;;      	 418b4670             	mov	eax, dword ptr [r14 + 0x70]
;;      	 83e801               	sub	eax, 1
;;      	 41894670             	mov	dword ptr [r14 + 0x70], eax
;;      	 498b4658             	mov	rax, qword ptr [r14 + 0x58]
;;      	 c1e810               	shr	eax, 0x10
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 4d8b5e10             	mov	r11, qword ptr [r14 + 0x10]
;;      	 498b0b               	mov	rcx, qword ptr [r11]
;;      	 4883ec0c             	sub	rsp, 0xc
;;      	 4c89f7               	mov	rdi, r14
;;      	 8b74240c             	mov	esi, dword ptr [rsp + 0xc]
;;      	 ba00000000           	mov	edx, 0
;;      	 ffd1                 	call	rcx
;;      	 4883c40c             	add	rsp, 0xc
;;      	 4883c404             	add	rsp, 4
;;      	 4c8b742448           	mov	r14, qword ptr [rsp + 0x48]
;;      	 89c0                 	mov	eax, eax
;;      	 498b4e50             	mov	rcx, qword ptr [r14 + 0x50]
;;      	 4801c1               	add	rcx, rax
;;      	 4881c124300200       	add	rcx, 0x23024
;;      	 480fbe01             	movsx	rax, byte ptr [rcx]
;;      	 f30f100543000000     	movss	xmm0, dword ptr [rip + 0x43]
;;      	 4883ec0c             	sub	rsp, 0xc
;;      	 f2440f103d3e000000   	
;; 				movsd	xmm15, qword ptr [rip + 0x3e]
;;      	 f2440f113c24         	movsd	qword ptr [rsp], xmm15
;;      	 f3440f103d27000000   	
;; 				movss	xmm15, dword ptr [rip + 0x27]
;;      	 f3440f117c2408       	movss	dword ptr [rsp + 8], xmm15
;;      	 488b44240c           	mov	rax, qword ptr [rsp + 0xc]
;;      	 415b                 	pop	r11
;;      	 4c8918               	mov	qword ptr [rax], r11
;;      	 448b1c24             	mov	r11d, dword ptr [rsp]
;;      	 4883c404             	add	rsp, 4
;;      	 44895808             	mov	dword ptr [rax + 8], r11d
;;      	 4883c450             	add	rsp, 0x50
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;  10c:	 0f0b                 	ud2	
;;  10e:	 0000                 	add	byte ptr [rax], al
;;  110:	 0000                 	add	byte ptr [rax], al
;;  112:	 0000                 	add	byte ptr [rax], al
;;  114:	 0000                 	add	byte ptr [rax], al
;;  116:	 0000                 	add	byte ptr [rax], al
;;  118:	 0000                 	add	byte ptr [rax], al
;;  11a:	 0000                 	add	byte ptr [rax], al
;;  11c:	 0000                 	add	byte ptr [rax], al
;;  11e:	 0000                 	add	byte ptr [rax], al
