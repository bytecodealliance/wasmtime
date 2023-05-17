;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo i32)  
        (local $bar i32)

        (i32.const 10)
        (local.set $foo)

        (i32.const 20)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        i32.mul
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 b80a000000           	mov	eax, 0xa
;;   1a:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1e:	 b814000000           	mov	eax, 0x14
;;   23:	 89442408             	mov	dword ptr [rsp + 8], eax
;;   27:	 8b442408             	mov	eax, dword ptr [rsp + 8]
;;   2b:	 8b4c240c             	mov	ecx, dword ptr [rsp + 0xc]
;;   2f:	 0fafc8               	imul	ecx, eax
;;   32:	 4889c8               	mov	rax, rcx
;;   35:	 4883c410             	add	rsp, 0x10
;;   39:	 5d                   	pop	rbp
;;   3a:	 c3                   	ret	
