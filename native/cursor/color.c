#include "internal.h"

#include <fcntl.h>
#include <poll.h>
#include <stdio.h>
#include <string.h>
#include <termios.h>
#include <unistd.h>

enum {
    KEEL_TERMINAL_COLOR_QUERY_TIMEOUT_MS = 100,
};

typedef struct {
    unsigned char background[3];
    unsigned char cursor[3];
    int have_background;
    int have_cursor;
} KeelTerminalColors;

static KeelTerminalColors terminal_colors;
static int terminal_colors_queried;

static int hex_digit(unsigned char value)
{
    if (value >= '0' && value <= '9')
        return value - '0';
    if (value >= 'a' && value <= 'f')
        return value - 'a' + 10;
    if (value >= 'A' && value <= 'F')
        return value - 'A' + 10;
    return -1;
}

static int parse_hex_component(const unsigned char *value, size_t length,
                               unsigned char *output)
{
    unsigned int parsed = 0;
    unsigned int maximum;
    size_t index;

    if (length == 0 || length > 4)
        return 0;
    for (index = 0; index < length; index++) {
        int digit = hex_digit(value[index]);
        if (digit < 0)
            return 0;
        parsed = (parsed << 4) | (unsigned int)digit;
    }
    maximum = (1u << (length * 4)) - 1u;
    *output = (unsigned char)((parsed * 255u + maximum / 2u) / maximum);
    return 1;
}

static int parse_rgb_value(const unsigned char *value, size_t length,
                           unsigned char output[3])
{
    size_t index;

    while (length > 0 && (value[length - 1] == ' ' || value[length - 1] == '\t'))
        length--;
    if (length >= 4 && memcmp(value, "rgb:", 4) == 0) {
        value += 4;
        length -= 4;
        for (index = 0; index < 3; index++) {
            const unsigned char *separator = memchr(value, '/', length);
            size_t component_length = separator == NULL ? length :
                                      (size_t)(separator - value);
            if (!parse_hex_component(value, component_length, &output[index]))
                return 0;
            if (separator == NULL)
                return index == 2;
            length -= component_length + 1;
            value = separator + 1;
        }
        return 0;
    }
    if (length == 7 && value[0] == '#') {
        for (index = 0; index < 3; index++) {
            if (!parse_hex_component(value + 1 + index * 2, 2, &output[index]))
                return 0;
        }
        return 1;
    }
    return 0;
}

static int parse_osc_color(const unsigned char *buffer, size_t length, int code,
                           unsigned char output[3])
{
    unsigned char prefix[8];
    size_t prefix_length;
    size_t index;

    prefix_length = (size_t)snprintf((char *)prefix, sizeof(prefix), "\033]%d;", code);
    if (prefix_length >= sizeof(prefix))
        return 0;
    for (index = 0; index + prefix_length <= length; index++) {
        size_t end;
        if (memcmp(buffer + index, prefix, prefix_length) != 0)
            continue;
        for (end = index + prefix_length; end < length; end++) {
            if (buffer[end] == '\a' ||
                (buffer[end] == '\033' && end + 1 < length && buffer[end + 1] == '\\'))
                return parse_rgb_value(buffer + index + prefix_length,
                                       end - index - prefix_length, output);
        }
    }
    return 0;
}

void keel_query_terminal_colors(void)
{
    static const unsigned char query[] = "\033]11;?\033\\\033]12;?\033\\";
    unsigned char response[1024];
    struct termios original_termios;
    struct termios probe_termios;
    size_t response_length = 0;
    int original_flags;
    int elapsed = 0;
    int termios_changed = 0;
    struct timeval query_started;

    if (terminal_colors_queried)
        return;
    terminal_colors_queried = 1;
    memset(&terminal_colors, 0, sizeof(terminal_colors));
    if (SHTTY < 0 || tcgetattr(SHTTY, &original_termios) < 0)
        return;
    probe_termios = original_termios;
    probe_termios.c_lflag &= ~(ICANON | ECHO | ECHOE | ECHOK | ECHONL);
    probe_termios.c_cc[VMIN] = 0;
    probe_termios.c_cc[VTIME] = 0;
    if (tcsetattr(SHTTY, TCSANOW, &probe_termios) < 0)
        return;
    termios_changed = 1;
    if (!keel_write_all(query, sizeof(query) - 1))
        goto restore_terminal;
    (void)gettimeofday(&query_started, NULL);
    original_flags = fcntl(SHTTY, F_GETFL, 0);
    if (original_flags < 0 || fcntl(SHTTY, F_SETFL, original_flags | O_NONBLOCK) < 0)
        goto restore_terminal;
    while (elapsed < KEEL_TERMINAL_COLOR_QUERY_TIMEOUT_MS &&
           (!terminal_colors.have_background || !terminal_colors.have_cursor)) {
        struct pollfd descriptor = {.fd = SHTTY, .events = POLLIN};
        unsigned char chunk[256];
        int wait = KEEL_TERMINAL_COLOR_QUERY_TIMEOUT_MS - elapsed;
        int result = poll(&descriptor, 1, wait);
        if (result <= 0)
            break;
        for (;;) {
            ssize_t count = read(SHTTY, chunk, sizeof(chunk));
            if (count <= 0)
                break;
            if (response_length + (size_t)count > sizeof(response))
                count = (ssize_t)(sizeof(response) - response_length);
            memcpy(response + response_length, chunk, (size_t)count);
            response_length += (size_t)count;
            if (response_length == sizeof(response))
                break;
        }
        if (!terminal_colors.have_background)
            terminal_colors.have_background =
                parse_osc_color(response, response_length, 11, terminal_colors.background);
        if (!terminal_colors.have_cursor)
            terminal_colors.have_cursor =
                parse_osc_color(response, response_length, 12, terminal_colors.cursor);
        elapsed = (int)keel_elapsed_milliseconds_since(&query_started);
    }
    (void)fcntl(SHTTY, F_SETFL, original_flags);

restore_terminal:
    if (termios_changed)
        (void)tcsetattr(SHTTY, TCSANOW, &original_termios);
}

static unsigned char blend_channel(unsigned char background, unsigned char cursor,
                                   double opacity)
{
    double value = (double)background + ((double)cursor - (double)background) * opacity;
    if (value <= 0.0)
        return 0;
    if (value >= 255.0)
        return 255;
    return (unsigned char)(value + 0.5);
}

int keel_cursor_blended_color(unsigned char color[3])
{
    double opacity;
    size_t index;

    if (!terminal_colors.have_background || !terminal_colors.have_cursor)
        return 0;
    opacity = keel_cursor_opacity();
    for (index = 0; index < 3; index++)
        color[index] = blend_channel(terminal_colors.background[index],
                                     terminal_colors.cursor[index], opacity);
    return 1;
}

int keel_terminal_background_color(unsigned char color[3])
{
    if (!terminal_colors.have_background)
        return 0;
    memcpy(color, terminal_colors.background, sizeof(terminal_colors.background));
    return 1;
}

void keel_restore_terminal_cursor_color(void)
{
    char sequence[64];
    int length;

    if (!terminal_colors.have_cursor)
        return;
    length = snprintf((char *)sequence, sizeof(sequence),
                      "\033]12;#%02x%02x%02x\033\\",
                      terminal_colors.cursor[0], terminal_colors.cursor[1],
                      terminal_colors.cursor[2]);
    if (length > 0 && (size_t)length < sizeof(sequence))
        (void)keel_write_all((const unsigned char *)sequence, (size_t)length);
}
