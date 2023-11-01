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
;;    d:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   16:	 4c893424             	mov	qword ptr [rsp], r14
;;   1a:	 48c7c001000000       	mov	rax, 1
;;   21:	 4889442408           	mov	qword ptr [rsp + 8], rax
;;   26:	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;   2b:	 4883f800             	cmp	rax, 0
;;   2f:	 b800000000           	mov	eax, 0
;;   34:	 400f94c0             	sete	al
;;   38:	 85c0                 	test	eax, eax
;;   3a:	 0f8526000000         	jne	0x66
;;   40:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   45:	 488b4c2410           	mov	rcx, qword ptr [rsp + 0x10]
;;   4a:	 480fafc8             	imul	rcx, rax
;;   4e:	 48894c2408           	mov	qword ptr [rsp + 8], rcx
;;   53:	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;   58:	 4883e801             	sub	rax, 1
;;   5c:	 4889442410           	mov	qword ptr [rsp + 0x10], rax
;;   61:	 e9c0ffffff           	jmp	0x26
;;   66:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   6b:	 4883c418             	add	rsp, 0x18
;;   6f:	 5d                   	pop	rbp
;;   70:	 c3                   	ret	
