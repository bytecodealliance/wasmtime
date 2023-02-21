;;! target = "x86_64"

(module
    (func (param i64) (param i64) (result i64)
	(local.get 0)
	(local.get 1)
	(i64.div_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;    d:	 48893424             	mov	qword ptr [rsp], rsi
;;   11:	 488b0c24             	mov	rcx, qword ptr [rsp]
;;   15:	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;   1a:	 4883f900             	cmp	rcx, 0
;;   1e:	 0f8502000000         	jne	0x26
;;   24:	 0f0b                 	ud2	
;;   26:	 4883f9ff             	cmp	rcx, -1
;;   2a:	 0f8515000000         	jne	0x45
;;   30:	 49bb0000000000000080 	
;; 				movabs	r11, 0x8000000000000000
;;   3a:	 4c39d8               	cmp	rax, r11
;;   3d:	 0f8502000000         	jne	0x45
;;   43:	 0f0b                 	ud2	
;;   45:	 4899                 	cqo	
;;   47:	 48f7f9               	idiv	rcx
;;   4a:	 4883c410             	add	rsp, 0x10
;;   4e:	 5d                   	pop	rbp
;;   4f:	 c3                   	ret	
