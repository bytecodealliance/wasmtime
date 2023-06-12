;;! target = "x86_64"

(module
    (func (result i32)
        (i64.const 9223372036854775807)
        (i64.eqz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 48b8ffffffffffffff7f 	
;; 				movabs	rax, 0x7fffffffffffffff
;;   16:	 4883f800             	cmp	rax, 0
;;   1a:	 b800000000           	mov	eax, 0
;;   1f:	 400f94c0             	sete	al
;;   23:	 4883c408             	add	rsp, 8
;;   27:	 5d                   	pop	rbp
;;   28:	 c3                   	ret	
