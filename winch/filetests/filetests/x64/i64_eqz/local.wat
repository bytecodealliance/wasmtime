;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo i64)

        (i64.const 2)
        (local.set $foo)

        (local.get $foo)
        (i64.eqz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 48c7c002000000       	mov	rax, 2
;;   1c:	 4889442408           	mov	qword ptr [rsp + 8], rax
;;   21:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   26:	 4883f800             	cmp	rax, 0
;;   2a:	 b800000000           	mov	eax, 0
;;   2f:	 400f94c0             	sete	al
;;   33:	 4883c410             	add	rsp, 0x10
;;   37:	 5d                   	pop	rbp
;;   38:	 c3                   	ret	
