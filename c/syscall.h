#pragma once

#include <stddef.h>
#include <stdint.h>

#define SYS_EXIT  0
#define SYS_READ  1
#define SYS_WRITE 2
#define SYS_OPENF 3
#define SYS_YIELD 4
#define SYS_FB_INFO 5
#define SYS_FB_MMAP 6
#define SYS_FB_SWAP 7
#define SYS_SIGNAL 8
#define SYS_RAISE  9
#define SYS_FORK 10
#define SYS_GETPID 11
#define SYS_WAITPID 12
#define SYS_MMAP 13
#define SYS_PIPE 14
#define SYS_DUP2 15
#define SYS_EXECVE 16
#define SYS_CLOSE 17
#define SYS_MUNMAP 18
#define SYS_EXIT_GROUP 19
#define SYS_MPROTECT 20
#define SYS_BRK 21
#define SYS_SLEEP 23


#define SIG_KILL   1
#define SIG_PAUSE  2
#define SIG_RESUME 3
#define SIG_PAGE_FAULT 4
#define SIG_USER1  20
#define SIG_USER2  21
#define SIG_USER3  22

#define O_READ   (1 << 0)
#define O_WRITE  (1 << 1)
#define O_APPEND (1 << 2)
#define O_CREATE (1 << 3)

#define PROT_NONE 0
#define PROT_READ 1
#define PROT_WRITE (1 << 1)
#define PROT_EXEC (1 << 2)

#define MAP_NONE 0
#define MAP_PRIVATE (1 << 0)
#define MAP_SHARED (1 << 1)
#define MAP_ANONYMOUS (1 << 2)
#define MAP_FIXED (1 << 3)
#define MAP_FIXED_NOREPLACE (1 << 4)


#define FD_INVALID  UINT64_MAX
#define ERR_SYSCALL UINT64_MAX

typedef void (*signal_handler_t)(void);

typedef enum {
    PIXEL_FORMAT_RGB = 0,
    PIXEL_FORMAT_BGR = 1,
    PIXEL_FORMAT_U8 = 2,
} pixel_format_t;

typedef struct {
    uint64_t byte_len;
    uint64_t width;
    uint64_t height;
    uint32_t pixel_format;
    uint64_t bytes_per_pixel;
    uint64_t stride;
} fb_info_t;

typedef uint64_t fd_t;

static inline uint64_t _syscall6(uint64_t id,
                                  uint64_t a1, uint64_t a2, uint64_t a3,
                                  uint64_t a4, uint64_t a5, uint64_t a6)
{
    register uint64_t r10 asm("r10") = a4;
    register uint64_t r8  asm("r8")  = a5;
    register uint64_t r9  asm("r9")  = a6;
    asm volatile (
        "syscall"
        : "+r"(r9)                              // r9 = in: offset, out: retval
        : "a"(id), "D"(a1), "S"(a2), "d"(a3), "r"(r10), "r"(r8)
        : "rcx", "r11", "memory"
    );
    return r9;
}

static inline uint64_t _syscall3(uint64_t id,
                                  uint64_t a1,
                                  uint64_t a2,
                                  uint64_t a3)
{
    register uint64_t r9 asm("r9") = 0;
    asm volatile (
        "syscall"
        : "+r"(r9)
        : "a"(id), "D"(a1), "S"(a2), "d"(a3)
        : "rcx", "r11", "memory"
    );
    return r9;
}

static inline uint64_t _syscall0_ret(uint64_t id) {
    uint64_t ret;
    register uint64_t r9 asm("r9") = 0;
    asm volatile (
        "syscall"
        : "=a"(ret), "+r"(r9)
        : "a"(id)
        : "rcx", "r11", "memory"
    );
    return (r9 == ERR_SYSCALL) ? UINT64_MAX : ret;
}


#define _syscall0(id)     _syscall3((id), 0, 0, 0)
#define _syscall1(id,a)   _syscall3((id), (uint64_t)(a), 0, 0)
#define _syscall2(id,a,b) _syscall3((id), (uint64_t)(a), (uint64_t)(b), 0)

static inline void sys_exit(void)
{
    _syscall0(SYS_EXIT);
    __builtin_unreachable();
}


static inline void sys_yield(void)
{
    _syscall0(SYS_YIELD);
}

static inline int sys_write(fd_t fd, const void *buf, size_t len)
{
    uint64_t err = _syscall3(SYS_WRITE,
                             (uint64_t)fd,
                             (uint64_t)(uintptr_t)buf,
                             (uint64_t)len);
    return (err == ERR_SYSCALL) ? -1 : (int)err;
}


static inline int sys_read(fd_t fd, void *buf, size_t len)
{
    uint64_t err = _syscall3(SYS_READ,
                             (uint64_t)fd,
                             (uint64_t)(uintptr_t)buf,
                             (uint64_t)len);
    return (err == ERR_SYSCALL) ? -1 : (int)err;
}

static inline fd_t sys_openf(const char *path, uint32_t flags)
{
    uint64_t err = _syscall2(SYS_OPENF,
                             (uint64_t)(uintptr_t)path,
                             (uint64_t)flags);
    return (err == ERR_SYSCALL) ? FD_INVALID : err;
}

static inline int sys_signal(uint64_t signum, signal_handler_t handler) {
    uint64_t err = _syscall2(SYS_SIGNAL, signum, (uint64_t)handler);
    return (err == ERR_SYSCALL) ? -1 : 0;
}

static inline int sys_raise(uint64_t pid, uint64_t signum) {
    uint64_t err = _syscall2(SYS_RAISE, pid, signum);
    return (err == ERR_SYSCALL) ? -1 : 0;
}

static inline uint64_t sys_fork(void) {
    uint64_t err = _syscall0(SYS_FORK);
    return (err == ERR_SYSCALL) ? UINT64_MAX : err;
}

static inline uint64_t sys_waitpid(uint64_t pid) {
    uint64_t err = _syscall1(SYS_WAITPID, pid);
    return (err == ERR_SYSCALL) ? -1 : 0;
}

static inline uint64_t sys_getpid(void) {
    uint64_t pid = 0;
    uint64_t err = _syscall1(SYS_GETPID, (uint64_t)(uintptr_t)&pid);
    return (err == ERR_SYSCALL) ? UINT64_MAX : pid;
}

/*pub fn mmap(
    process: &mut Process,
    addr: u64,
    len: u64,
    prot: ProtFlags,
    flags: MapFlags,
    fd: u64,
    offset: u64,
) */

static inline void* sys_mmap(void* addr, size_t length, uint64_t prot, uint64_t flags, fd_t fd, uint64_t offset)
{
    void* mapped_addr = NULL;
    uint64_t err = _syscall6(SYS_MMAP,
                             (uint64_t)(uintptr_t)addr,
                             (uint64_t)length,
                             prot,
                             flags,
                             (uint64_t)fd,
                             offset);
    return (err == ERR_SYSCALL) ? NULL : (void*)(uintptr_t)err;
}

static inline int sys_pipe(fd_t *read_fd, fd_t *write_fd)
{
    fd_t fds[2] = {FD_INVALID, FD_INVALID};
    uint64_t err = _syscall1(SYS_PIPE, (uint64_t)(uintptr_t)fds);
    if (err == ERR_SYSCALL) {
        return -1;
    } else {
        *read_fd = fds[0];
        *write_fd = fds[1];
        return 0;
    }
}

static inline int sys_dup2(fd_t oldfd, fd_t newfd)
{
    uint64_t err = _syscall2(SYS_DUP2, (uint64_t)oldfd, (uint64_t)newfd);
    return (err == ERR_SYSCALL) ? -1 : 0;
}

static inline int sys_execve(const char *path, char *const argv[], char *const envp[])
{
    uint64_t err = _syscall3(SYS_EXECVE,
                             (uint64_t)(uintptr_t)path,
                             (uint64_t)(uintptr_t)argv,
                             (uint64_t)(uintptr_t)envp);
    return (err == ERR_SYSCALL) ? -1 : 0;
}

static inline int sys_close(fd_t fd)
{
    uint64_t err = _syscall1(SYS_CLOSE, (uint64_t)fd);
    return (err == ERR_SYSCALL) ? -1 : 0;
}

static inline int sys_munmap(void* addr, size_t length)
{
    uint64_t err = _syscall2(SYS_MUNMAP, (uint64_t)(uintptr_t)addr, (uint64_t)length);
    return (err == ERR_SYSCALL) ? -1 : 0;
}

static inline void sys_exit_group(uint8_t status)
{
    _syscall1(SYS_EXIT_GROUP, (uint64_t)status);
    __builtin_unreachable();
}

static inline void sys_sleep(uint64_t ms)
{
    _syscall1(SYS_SLEEP, ms);
}

static inline int sys_mprotect(void* addr, size_t length, uint64_t prot)
{
    uint64_t err = _syscall3(SYS_MPROTECT, (uint64_t)(uintptr_t)addr, (uint64_t)length, prot);
    return (err == ERR_SYSCALL) ? -1 : 0;
}

static inline int sys_fb_info(fb_info_t *info)
{
    uint64_t err = _syscall1(SYS_FB_INFO, (uint64_t)(uintptr_t)info);
    return (err == ERR_SYSCALL) ? -1 : 0;
}


static inline void* sys_fb_mmap(void)
{
    void* addr = NULL;
    uint64_t err = _syscall1(SYS_FB_MMAP, (uint64_t)(uintptr_t)&addr);
    return (err == ERR_SYSCALL) ? NULL : addr;
}


static inline int sys_fb_swap(void)
{
    uint64_t err = _syscall0(SYS_FB_SWAP);
    return (err == ERR_SYSCALL) ? -1 : 0;
}
