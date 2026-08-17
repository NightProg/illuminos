#include "c/syscall.h"
#include "sys/types.h"
#include <stdint.h>

static void itoa(long value, char* buf, int base) {
    char tmp[32];
    int i = 0;
    int neg = 0;

    if (value == 0) {
        buf[0] = '0';
        buf[1] = 0;
        return;
    }

    if (value < 0 && base == 10) {
        neg = 1;
        value = -value;
    }

    while (value > 0) {
        int digit = value % base;
        tmp[i++] = (digit < 10) ? '0' + digit : 'a' + digit - 10;
        value /= base;
    }

    int j = 0;
    if (neg) buf[j++] = '-';

    while (i--) {
        buf[j++] = tmp[i];
    }

    buf[j] = 0;
}
#include <stdarg.h>


void printf(const char* fmt, ...) {
    va_list args;
    va_start(args, fmt);

    char buffer[256];
    int bi = 0;

    for (int i = 0; fmt[i]; i++) {
        if (fmt[i] == '%') {
            i++;

            if (fmt[i] == 's') {
                char* s = va_arg(args, char*);
                while (*s) buffer[bi++] = *s++;
            }
            else if (fmt[i] == 'd') {
                char tmp[32];
                itoa(va_arg(args, int), tmp, 10);
                for (char* t = tmp; *t; t++) buffer[bi++] = *t;
            }
            else if (fmt[i] == 'x') {
                char tmp[32];
                itoa(va_arg(args, int), tmp, 16);
                for (char* t = tmp; *t; t++) buffer[bi++] = *t;
            }
            else if (fmt[i] == 'c') {
                buffer[bi++] = (char)va_arg(args, int);
            } else if (fmt[i] == 'p') {
                char tmp[32];
                itoa((uintptr_t)va_arg(args, void*), tmp, 16);
                for (char* t = tmp; *t; t++) buffer[bi++] = *t;
            }
            else {
                buffer[bi++] = '%';
                buffer[bi++] = fmt[i];
            }
        } else {
            buffer[bi++] = fmt[i];
        }

        // flush si buffer plein
        if (bi > 240) {
            sys_write(1, buffer, bi);
            bi = 0;
        }
    }

    if (bi > 0) {
        sys_write(1, buffer, bi);
    }

    va_end(args);
}

void _putchar(char c) {
    sys_write(1, &c, 1);
}
void putstr(const char *s) {
    size_t len = 0;
    while (s[len]) len++;
    sys_write(1, s, len);
}

void putint(int n)
{
    char buf[12];
    int i = 0;

    if (n == 0) {
        sys_write(1, "0", 1);
        return;
    }

    while (n > 0) {
        buf[11 - i++] = '0' + (n % 10);
        n /= 10;
    }

    sys_write(1, buf + (12 - i), i);
}


int main(int argc, char *argv[]) {
    int i = argc;
    for (int j = 0; j < argc; j++) {
        printf("arg[%d] = %s\n", j, argv[j]);
    }

    fd_t fd = sys_openf("/dev/tty", O_READ);
    if (fd != FD_INVALID) {
        char buf[5];
        int n = sys_read(fd, buf, sizeof(buf) - 1);
        if (n > 0) {
            buf[n] = 0;
            printf("Read from /dev/tty: %s\n", buf);
        }
        printf("read ttbateauy");
        sys_close(fd);
    } else {
        printf("Failedt to open /dev/tty\n");
        }
    return 0;
}
