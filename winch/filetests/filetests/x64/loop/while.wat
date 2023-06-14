;;! target = "x86_64"
(module
  (func (export "while-") (param i64) (result i64)
    (local i64)
    (local.set 1 (i64.const 1))
    (block
      (loop
        (br_if 1 (i64.eqz (local.get 0)))
        (local.set 1 (i64.mul (local.get 0) (local.get 1)))
        (local.set 0 (i64.sub (local.get 0) (i64.const 1)))
        (br 0)
      )
    )
    (local.get 1)
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 48897c2410           	mov	qword ptr [rsp + 0x10], rdi
;;    d:	 4c893424             	mov	qword ptr [rsp], r14
;;   11:	 48c7c001000000       	mov	rax, 1
;;   18:	 4889442408           	mov	qword ptr [rsp + 8], rax
;;   1d:	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;   22:	 4883f800             	cmp	rax, 0
;;   26:	 b800000000           	mov	eax, 0
;;   2b:	 400f94c0             	sete	al
;;   2f:	 50                   	push	rax
;;   30:	 59                   	pop	rcx
;;   31:	 85c9                 	test	ecx, ecx
;;   33:	 0f8526000000         	jne	0x5f
;;   39:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   3e:	 488b4c2410           	mov	rcx, qword ptr [rsp + 0x10]
;;   43:	 480fafc8             	imul	rcx, rax
;;   47:	 48894c2408           	mov	qword ptr [rsp + 8], rcx
;;   4c:	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;   51:	 4883e801             	sub	rax, 1
;;   55:	 4889442410           	mov	qword ptr [rsp + 0x10], rax
;;   5a:	 e9beffffff           	jmp	0x1d
;;   5f:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   64:	 4883c418             	add	rsp, 0x18
;;   68:	 5d                   	pop	rbp
;;   69:	 c3                   	ret	
