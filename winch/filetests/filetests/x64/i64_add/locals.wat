;;! target = "x86_64"

(module
    (func (result i64)
        (local $foo i64)  
        (local $bar i64)

        (i64.const 10)
        (local.set $foo)

        (i64.const 20)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        i64.add
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 4531db               	xor	r11d, r11d
;;    b:	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;   10:	 4c891c24             	mov	qword ptr [rsp], r11
;;   14:	 48c7c00a000000       	mov	rax, 0xa
;;   1b:	 4889442408           	mov	qword ptr [rsp + 8], rax
;;   20:	 48c7c014000000       	mov	rax, 0x14
;;   27:	 48890424             	mov	qword ptr [rsp], rax
;;   2b:	 488b0424             	mov	rax, qword ptr [rsp]
;;   2f:	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;   34:	 4801c1               	add	rcx, rax
;;   37:	 4889c8               	mov	rax, rcx
;;   3a:	 4883c410             	add	rsp, 0x10
;;   3e:	 5d                   	pop	rbp
;;   3f:	 c3                   	ret	
