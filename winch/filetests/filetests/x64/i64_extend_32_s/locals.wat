;;! target = "x86_64"

(module
    (func (result i64)
        (local i64)

        (local.get 0)
        (i64.extend32_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   1a:	 4863c0               	movsxd	rax, eax
;;   1d:	 4883c410             	add	rsp, 0x10
;;   21:	 5d                   	pop	rbp
;;   22:	 c3                   	ret	
