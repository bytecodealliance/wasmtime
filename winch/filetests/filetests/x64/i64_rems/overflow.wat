;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0x8000000000000000)
	(i64.const -1)
	(i64.rem_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c1ffffffff       	mov	rcx, 0xffffffffffffffff
;;   13:	 48b80000000000000080 	
;; 				movabs	rax, 0x8000000000000000
;;   1d:	 4899                 	cqo	
;;   1f:	 4883f9ff             	cmp	rcx, -1
;;   23:	 0f850a000000         	jne	0x33
;;   29:	 ba00000000           	mov	edx, 0
;;   2e:	 e903000000           	jmp	0x36
;;   33:	 48f7f9               	idiv	rcx
;;   36:	 4889d0               	mov	rax, rdx
;;   39:	 4883c408             	add	rsp, 8
;;   3d:	 5d                   	pop	rbp
;;   3e:	 c3                   	ret	
