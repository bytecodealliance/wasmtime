;;! target = "x86_64"

(module
    (func (result i64)
        i32.const 1
        i64.extend_i32_s
        block
        end
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b801000000           	mov	eax, 1
;;   11:	 4863c0               	movsxd	rax, eax
;;   14:	 50                   	push	rax
;;   15:	 58                   	pop	rax
;;   16:	 4883c408             	add	rsp, 8
;;   1a:	 5d                   	pop	rbp
;;   1b:	 c3                   	ret	
