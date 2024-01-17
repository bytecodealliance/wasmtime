;;! target = "x86_64"
(module
  (func (export "as-if-then") (result i32)
    (if (result i32) (i32.const 1) (then (loop (result i32) (i32.const 1))) (else (i32.const 2)))
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b801000000           	mov	eax, 1
;;      	 85c0                 	test	eax, eax
;;      	 0f840a000000         	je	0x23
;;   19:	 b801000000           	mov	eax, 1
;;      	 e905000000           	jmp	0x28
;;   23:	 b802000000           	mov	eax, 2
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
