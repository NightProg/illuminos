extern main
global _start

_start:
    mov rdi, [rsp]      ; argc
    lea rsi, [rsp+8]     ; argv
    call main

    mov rdi, rax
    mov rax, 0        
    syscall