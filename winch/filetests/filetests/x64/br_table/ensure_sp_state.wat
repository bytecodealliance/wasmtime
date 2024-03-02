;;! target = "x86_64"

(module
  (func (export "") (result i32)
    block (result i32)
       i32.const 0
    end
    i32.const 0
    i32.const 0
    br_table 0
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c344000000       	add	r11, 0x44
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8788000000         	ja	0xa3
;;   1b:	 4883ec30             	sub	rsp, 0x30
;;      	 48891c24             	mov	qword ptr [rsp], rbx
;;      	 4c89642408           	mov	qword ptr [rsp + 8], r12
;;      	 4c896c2410           	mov	qword ptr [rsp + 0x10], r13
;;      	 4c89742418           	mov	qword ptr [rsp + 0x18], r14
;;      	 4c897c2420           	mov	qword ptr [rsp + 0x20], r15
;;      	 4989fe               	mov	r14, rdi
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2438           	mov	qword ptr [rsp + 0x38], rdi
;;      	 4889742430           	mov	qword ptr [rsp + 0x30], rsi
;;      	 b800000000           	mov	eax, 0
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 b900000000           	mov	ecx, 0
;;      	 b800000000           	mov	eax, 0
;;      	 ba00000000           	mov	edx, 0
;;      	 39ca                 	cmp	edx, ecx
;;      	 0f42ca               	cmovb	ecx, edx
;;      	 4c8d1d0a000000       	lea	r11, [rip + 0xa]
;;      	 4963148b             	movsxd	rdx, dword ptr [r11 + rcx*4]
;;      	 4901d3               	add	r11, rdx
;;      	 41ffe3               	jmp	r11
;;   79:	 0400                 	add	al, 0
;;      	 0000                 	add	byte ptr [rax], al
;;      	 4883c404             	add	rsp, 4
;;      	 4883c410             	add	rsp, 0x10
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   a3:	 0f0b                 	ud2	
