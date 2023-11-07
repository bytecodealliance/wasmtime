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
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec20             	sub	rsp, 0x20
;;    8:	 48897c2418           	mov	qword ptr [rsp + 0x18], rdi
;;    d:	 4531db               	xor	r11d, r11d
;;   10:	 4c895c2410           	mov	qword ptr [rsp + 0x10], r11
;;   15:	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;   1a:	 4c893424             	mov	qword ptr [rsp], r14
;;   1e:	 48c7c001000000       	mov	rax, 1
;;   25:	 4889442410           	mov	qword ptr [rsp + 0x10], rax
;;   2a:	 48c7c002000000       	mov	rax, 2
;;   31:	 4889442408           	mov	qword ptr [rsp + 8], rax
;;   36:	 488b442418           	mov	rax, qword ptr [rsp + 0x18]
;;   3b:	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;   40:	 4839c1               	cmp	rcx, rax
;;   43:	 b900000000           	mov	ecx, 0
;;   48:	 400f97c1             	seta	cl
;;   4c:	 85c9                 	test	ecx, ecx
;;   4e:	 0f8526000000         	jne	0x7a
;;   54:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   59:	 488b4c2410           	mov	rcx, qword ptr [rsp + 0x10]
;;   5e:	 480fafc8             	imul	rcx, rax
;;   62:	 48894c2410           	mov	qword ptr [rsp + 0x10], rcx
;;   67:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   6c:	 4883c001             	add	rax, 1
;;   70:	 4889442408           	mov	qword ptr [rsp + 8], rax
;;   75:	 e9bcffffff           	jmp	0x36
;;   7a:	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;   7f:	 4883c420             	add	rsp, 0x20
;;   83:	 5d                   	pop	rbp
;;   84:	 c3                   	ret	
