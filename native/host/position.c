#include "internal.h"

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <poll.h>
#include <string.h>
#include <termios.h>
#include <unistd.h>

enum {
    KEEL_CURSOR_QUERY_TIMEOUT_MS = 10,
    KEEL_CURSOR_QUERY_BUFFER_SIZE = 1024,
};

static int find_cursor_response(const unsigned char *buffer, size_t length,
                                size_t *response_start, size_t *response_end,
                                unsigned int *row)
{
    size_t start;

    for (start = 0; start + 3 < length; start++) {
        size_t cursor;
        unsigned int parsed_row = 0;
        unsigned int parsed_column = 0;

        if (buffer[start] != '\033' || buffer[start + 1] != '[')
            continue;
        cursor = start + 2;
        if (cursor >= length || buffer[cursor] < '0' || buffer[cursor] > '9')
            continue;
        while (cursor < length && buffer[cursor] >= '0' && buffer[cursor] <= '9') {
            unsigned int digit = (unsigned int)(buffer[cursor] - '0');
            if (parsed_row > (UINT_MAX - digit) / 10u)
                break;
            parsed_row = parsed_row * 10u + digit;
            cursor++;
        }
        if (cursor >= length || buffer[cursor] != ';' || parsed_row == 0)
            continue;
        cursor++;
        if (cursor >= length || buffer[cursor] < '0' || buffer[cursor] > '9')
            continue;
        while (cursor < length && buffer[cursor] >= '0' && buffer[cursor] <= '9') {
            unsigned int digit = (unsigned int)(buffer[cursor] - '0');
            if (parsed_column <= (UINT_MAX - digit) / 10u)
                parsed_column = parsed_column * 10u + digit;
            cursor++;
        }
        if (cursor >= length || buffer[cursor] != 'R' || parsed_column == 0)
            continue;
        *response_start = start;
        *response_end = cursor + 1;
        *row = parsed_row;
        return 1;
    }
    return 0;
}

static int read_cursor_row(uint16_t *row)
{
    unsigned char input[KEEL_CURSOR_QUERY_BUFFER_SIZE];
    size_t input_length = 0;
    size_t response_start = 0;
    size_t response_end = 0;
    size_t preserved_length = 0;
    unsigned int parsed_row = 0;
    int original_flags;
    int flags_changed = 0;
    int success = 0;
    struct termios original_termios;
    struct termios query_termios;
    struct pollfd descriptor = {.fd = SHTTY, .events = POLLIN};
    struct timeval query_started;
    int elapsed;

    if (SHTTY < 0 || row == NULL || tcgetattr(SHTTY, &original_termios) < 0)
        return 0;
    query_termios = original_termios;
    query_termios.c_lflag &= ~(ICANON | ECHO | ECHOE | ECHOK | ECHONL);
    query_termios.c_cc[VMIN] = 0;
    query_termios.c_cc[VTIME] = 0;
    if (tcsetattr(SHTTY, TCSANOW, &query_termios) < 0)
        return 0;

    original_flags = fcntl(SHTTY, F_GETFL, 0);
    if (original_flags < 0 || fcntl(SHTTY, F_SETFL, original_flags | O_NONBLOCK) < 0)
        goto restore_termios;
    flags_changed = 1;
    if (!keel_write_all((const unsigned char *)"\033[6n", 4))
        goto restore_flags;
    (void)gettimeofday(&query_started, NULL);

    while ((elapsed = (int)keel_elapsed_milliseconds_since(&query_started)) <
           KEEL_CURSOR_QUERY_TIMEOUT_MS) {
        ssize_t count;
        int result = poll(&descriptor, 1, KEEL_CURSOR_QUERY_TIMEOUT_MS - elapsed);

        if (result <= 0)
            break;
        if (descriptor.revents & (POLLERR | POLLHUP | POLLNVAL))
            break;
        if (input_length == sizeof(input))
            break;
        count = read(SHTTY, input + input_length, sizeof(input) - input_length);
        if (count < 0 && errno == EINTR)
            continue;
        if (count <= 0)
            break;
        input_length += (size_t)count;
        if (find_cursor_response(input, input_length, &response_start, &response_end,
                                 &parsed_row)) {
            preserved_length = response_start + input_length - response_end;
            memmove(input + response_start, input + response_end,
                    input_length - response_end);
            *row = (uint16_t)(parsed_row > UINT16_MAX ? UINT16_MAX : parsed_row);
            success = 1;
            break;
        }
    }

restore_flags:
    if (flags_changed)
        (void)fcntl(SHTTY, F_SETFL, original_flags);
restore_termios:
    (void)tcsetattr(SHTTY, TCSANOW, &original_termios);
    if (!success)
        preserved_length = input_length;
    if (preserved_length > 0)
        ungetbytes((char *)input, (int)preserved_length);
    return success;
}

int keel_capture_zle_line_origin(void)
{
    uint16_t terminal_row;

    if (!read_cursor_row(&terminal_row) || terminal_row == 0)
        return 0;
    return terminal_row;
}
