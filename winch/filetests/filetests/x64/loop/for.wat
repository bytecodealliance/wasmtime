;;! target = "x86_64"
(module
  (func (export "for-") (param i64) (result i64)
    (local i64 i64)
    (local.set 1 (i64.const 1))
    (local.set 2 (i64.const 2))
    (block
      (loop
        (br_if 1 (i64.gt_u (local.get 2) (local.get 0)))
        (local.set 1 (i64.mul (local.get 1) (local.get 2)))
        (local.set 2 (i64.add (local.get 2) (i64.const 1)))
        (br 0)
      )
    )
    (local.get 1)
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec20             	sub	rsp, 0x20
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f877d000000         	ja	0x95
;;   18:	 48897c2418           	mov	qword ptr [rsp + 0x18], rdi
;;      	 4531db               	xor	r11d, r11d
;;      	 4c895c2410           	mov	qword ptr [rsp + 0x10], r11
;;      	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c001000000       	mov	rax, 1
;;      	 4889442410           	mov	qword ptr [rsp + 0x10], rax
;;      	 48c7c002000000       	mov	rax, 2
;;      	 4889442408           	mov	qword ptr [rsp + 8], rax
;;      	 488b442418           	mov	rax, qword ptr [rsp + 0x18]
;;      	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;      	 4839c1               	cmp	rcx, rax
;;      	 b900000000           	mov	ecx, 0
;;      	 400f97c1             	seta	cl
;;      	 85c9                 	test	ecx, ecx
;;      	 0f8526000000         	jne	0x8a
;;   64:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;      	 488b4c2410           	mov	rcx, qword ptr [rsp + 0x10]
;;      	 480fafc8             	imul	rcx, rax
;;      	 48894c2410           	mov	qword ptr [rsp + 0x10], rcx
;;      	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;      	 4883c001             	add	rax, 1
;;      	 4889442408           	mov	qword ptr [rsp + 8], rax
;;      	 e9bcffffff           	jmp	0x46
;;   8a:	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;      	 4883c420             	add	rsp, 0x20
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   95:	 0f0b                 	ud2	
