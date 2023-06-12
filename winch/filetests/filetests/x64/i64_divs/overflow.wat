;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const 0x8000000000000000)
	(i64.const -1)
	(i64.div_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48c7c1ffffffff       	mov	rcx, 0xffffffffffffffff
;;   13:	 48b80000000000000080 	
;; 				movabs	rax, 0x8000000000000000
;;   1d:	 4883f900             	cmp	rcx, 0
;;   21:	 0f840b000000         	je	0x32
;;   27:	 4899                 	cqo	
;;   29:	 48f7f9               	idiv	rcx
;;   2c:	 4883c408             	add	rsp, 8
;;   30:	 5d                   	pop	rbp
;;   31:	 c3                   	ret	
;;   32:	 0f0b                 	ud2	
