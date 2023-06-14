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
;;    d:	 48c744241000000000   	
;; 				mov	qword ptr [rsp + 0x10], 0
;;   16:	 4c893424             	mov	qword ptr [rsp], r14
;;   1a:	 48c7c001000000       	mov	rax, 1
;;   21:	 4889442410           	mov	qword ptr [rsp + 0x10], rax
;;   26:	 48c7c002000000       	mov	rax, 2
;;   2d:	 4889442408           	mov	qword ptr [rsp + 8], rax
;;   32:	 488b442418           	mov	rax, qword ptr [rsp + 0x18]
;;   37:	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;   3c:	 4839c1               	cmp	rcx, rax
;;   3f:	 b900000000           	mov	ecx, 0
;;   44:	 400f97c1             	seta	cl
;;   48:	 85c9                 	test	ecx, ecx
;;   4a:	 0f8526000000         	jne	0x76
;;   50:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   55:	 488b4c2410           	mov	rcx, qword ptr [rsp + 0x10]
;;   5a:	 480fafc8             	imul	rcx, rax
;;   5e:	 48894c2410           	mov	qword ptr [rsp + 0x10], rcx
;;   63:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   68:	 4883c001             	add	rax, 1
;;   6c:	 4889442408           	mov	qword ptr [rsp + 8], rax
;;   71:	 e9bcffffff           	jmp	0x32
;;   76:	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;   7b:	 4883c420             	add	rsp, 0x20
;;   7f:	 5d                   	pop	rbp
;;   80:	 c3                   	ret	
