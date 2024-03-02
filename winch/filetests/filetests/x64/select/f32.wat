;;! target = "x86_64"

(module
  (func (export "select-f32") (param f32 f32 i32) (result f32)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4c8b5f08             	mov	r11, qword ptr [rdi + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c350000000       	add	r11, 0x50
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f877c000000         	ja	0x97
;;   1b:	 4883ec30             	sub	rsp, 0x30
;;      	 48891c24             	mov	qword ptr [rsp], rbx
;;      	 4c89642408           	mov	qword ptr [rsp + 8], r12
;;      	 4c896c2410           	mov	qword ptr [rsp + 0x10], r13
;;      	 4c89742418           	mov	qword ptr [rsp + 0x18], r14
;;      	 4c897c2420           	mov	qword ptr [rsp + 0x20], r15
;;      	 4989fe               	mov	r14, rdi
;;      	 4883ec20             	sub	rsp, 0x20
;;      	 48897c2448           	mov	qword ptr [rsp + 0x48], rdi
;;      	 4889742440           	mov	qword ptr [rsp + 0x40], rsi
;;      	 f30f1144243c         	movss	dword ptr [rsp + 0x3c], xmm0
;;      	 f30f114c2438         	movss	dword ptr [rsp + 0x38], xmm1
;;      	 89542434             	mov	dword ptr [rsp + 0x34], edx
;;      	 8b442434             	mov	eax, dword ptr [rsp + 0x34]
;;      	 f30f10442438         	movss	xmm0, dword ptr [rsp + 0x38]
;;      	 f30f104c243c         	movss	xmm1, dword ptr [rsp + 0x3c]
;;      	 83f800               	cmp	eax, 0
;;      	 0f8404000000         	je	0x75
;;   71:	 f20f10c1             	movsd	xmm0, xmm1
;;      	 4883c420             	add	rsp, 0x20
;;      	 488b1c24             	mov	rbx, qword ptr [rsp]
;;      	 4c8b642408           	mov	r12, qword ptr [rsp + 8]
;;      	 4c8b6c2410           	mov	r13, qword ptr [rsp + 0x10]
;;      	 4c8b742418           	mov	r14, qword ptr [rsp + 0x18]
;;      	 4c8b7c2420           	mov	r15, qword ptr [rsp + 0x20]
;;      	 4883c430             	add	rsp, 0x30
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   97:	 0f0b                 	ud2	
