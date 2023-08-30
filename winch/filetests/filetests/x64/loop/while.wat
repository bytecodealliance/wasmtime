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
;;   2f:	 85c0                 	test	eax, eax
;;   31:	 0f8526000000         	jne	0x5d
;;   37:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   3c:	 488b4c2410           	mov	rcx, qword ptr [rsp + 0x10]
;;   41:	 480fafc8             	imul	rcx, rax
;;   45:	 48894c2408           	mov	qword ptr [rsp + 8], rcx
;;   4a:	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;   4f:	 4883e801             	sub	rax, 1
;;   53:	 4889442410           	mov	qword ptr [rsp + 0x10], rax
;;   58:	 e9c0ffffff           	jmp	0x1d
;;   5d:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   62:	 4883c418             	add	rsp, 0x18
;;   66:	 5d                   	pop	rbp
;;   67:	 c3                   	ret	
