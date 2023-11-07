;;! target = "x86_64"
(module
  (func (export "as-if-then") (result i32)
    (if (result i32) (i32.const 1) (then (loop (result i32) (i32.const 1))) (else (i32.const 2)))
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 85c0                 	test	eax, eax
;;   13:	 0f840a000000         	je	0x23
;;   19:	 b801000000           	mov	eax, 1
;;   1e:	 e905000000           	jmp	0x28
;;   23:	 b802000000           	mov	eax, 2
;;   28:	 4883c408             	add	rsp, 8
;;   2c:	 5d                   	pop	rbp
;;   2d:	 c3                   	ret	
