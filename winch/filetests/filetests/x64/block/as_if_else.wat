;;! target = "x86_64"
(module
  (func (export "as-if-else") (result i32)
      (if (result i32) (i32.const 1) (then (i32.const 2)) (else (block (result i32) (i32.const 1))))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 85c0                 	test	eax, eax
;;   13:	 0f840c000000         	je	0x25
;;   19:	 48c7c002000000       	mov	rax, 2
;;   20:	 e907000000           	jmp	0x2c
;;   25:	 48c7c001000000       	mov	rax, 1
;;   2c:	 4883c408             	add	rsp, 8
;;   30:	 5d                   	pop	rbp
;;   31:	 c3                   	ret	
