;;! target = "x86_64"

(module
    (func (result i32)
        (f64.const 1.0)
        (i32.trunc_f64_u)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 f20f100d54000000     	movsd	xmm1, qword ptr [rip + 0x54]
;;   14:	 49bb000000000000e041 	
;; 				movabs	r11, 0x41e0000000000000
;;   1e:	 664d0f6efb           	movq	xmm15, r11
;;   23:	 66410f2ecf           	ucomisd	xmm1, xmm15
;;   28:	 0f8315000000         	jae	0x43
;;   2e:	 0f8a30000000         	jp	0x64
;;   34:	 f20f2cc1             	cvttsd2si	eax, xmm1
;;   38:	 83f800               	cmp	eax, 0
;;   3b:	 0f8d1d000000         	jge	0x5e
;;   41:	 0f0b                 	ud2	
;;   43:	 0f28c1               	movaps	xmm0, xmm1
;;   46:	 f2410f5cc7           	subsd	xmm0, xmm15
;;   4b:	 f20f2cc0             	cvttsd2si	eax, xmm0
;;   4f:	 83f800               	cmp	eax, 0
;;   52:	 0f8c0e000000         	jl	0x66
;;   58:	 81c000000080         	add	eax, 0x80000000
;;   5e:	 4883c408             	add	rsp, 8
;;   62:	 5d                   	pop	rbp
;;   63:	 c3                   	ret	
;;   64:	 0f0b                 	ud2	
;;   66:	 0f0b                 	ud2	
;;   68:	 0000                 	add	byte ptr [rax], al
;;   6a:	 0000                 	add	byte ptr [rax], al
;;   6c:	 0000                 	add	byte ptr [rax], al
