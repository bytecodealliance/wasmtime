;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo i32)
        (local $bar i32)

        (i32.const 2)
        (local.set $foo)
        (i32.const 3)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i32.ge_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 b802000000           	mov	eax, 2
;;   1a:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1e:	 b803000000           	mov	eax, 3
;;   23:	 89442408             	mov	dword ptr [rsp + 8], eax
;;   27:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   2b:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   2f:	 39c1                 	cmp	ecx, eax
;;   31:	 b900000000           	mov	ecx, 0
;;   36:	 400f93c1             	setae	cl
;;   3a:	 4889c8               	mov	rax, rcx
;;   3d:	 4883c410             	add	rsp, 0x10
;;   41:	 5d                   	pop	rbp
;;   42:	 c3                   	ret	
