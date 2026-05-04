.global switch_to

switch_to:
    test rdi, rdi
    jz .restore_next

    mov [rdi + 0x00], r15
    mov [rdi + 0x08], r14
    mov [rdi + 0x10], r13
    mov [rdi + 0x18], r12
    mov [rdi + 0x20], r11
    mov [rdi + 0x28], r10
    mov [rdi + 0x30], r9
    mov [rdi + 0x38], r8
    mov [rdi + 0x40], rbx
    mov [rdi + 0x48], rbp
    pushfq
    pop  [rdi + 0x50]
    mov [rdi + 0x58], rax
    mov rax, [rsp]
    mov [rdi + 0x60], rax
    lea rax, [rsp + 8]
    mov [rdi + 0x68], rax

.restore_next:
    mov r15, [rsi + 0x00]
    mov r14, [rsi + 0x08]
    mov r13, [rsi + 0x10]
    mov r12, [rsi + 0x18]
    mov r11, [rsi + 0x20]
    mov r10, [rsi + 0x28]
    mov r9,  [rsi + 0x30]
    mov r8,  [rsi + 0x38]
    mov rbx, [rsi + 0x40]
    mov rbp, [rsi + 0x48]
    push [rsi + 0x50]
    popfq
    mov rsp, [rsi + 0x68]
    push [rsi + 0x60]
    mov rax, [rsi + 0x58]
    ret
