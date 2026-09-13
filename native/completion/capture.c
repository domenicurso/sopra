#include "../keel_zsh_module_internal.h"

#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#define KEEL_COMPLETION_LIMIT 16384u
#define KEEL_ZSH_META 0x83u

static unsigned char capture_buffer[KEEL_MAX_HOST_BYTES];
static size_t capture_length;
static size_t capture_count;
static int capture_active;

static int append_byte(unsigned char byte)
{
    if (capture_length == sizeof(capture_buffer))
        return 0;
    capture_buffer[capture_length++] = byte;
    return 1;
}

static int append_decoded(const char *value, size_t length)
{
    size_t index;

    if (value == NULL)
        return 1;
    for (index = 0; index < length; index++) {
        unsigned char byte = (unsigned char)value[index];

        if (byte == KEEL_ZSH_META && index + 1 < length)
            byte = (unsigned char)value[++index] ^ 32u;
        if (byte == 0 || byte == 0x1d || byte == 0x1e || byte == 0x1f ||
            byte == '\r' || byte == '\n')
            return 0;
        if (!append_byte(byte))
            return 0;
    }
    return 1;
}

static const char *description_for(const char *original, const char *display)
{
    const char *rest;
    size_t original_length;

    if (display == NULL || display[0] == '\0')
        return NULL;
    if (original == NULL)
        return display;
    original_length = strlen(original);
    if (strncmp(display, original, original_length) != 0)
        return display;
    rest = display + original_length;
    while (*rest == ' ' || *rest == '\t')
        rest++;
    if (rest[0] == ':') {
        rest++;
        while (*rest == ' ' || *rest == '\t')
            rest++;
        return *rest == '\0' ? NULL : rest;
    }
    if (rest[0] == '-' && rest[1] == '-' && rest[2] == ' ')
        return rest + 3;
    return *rest == '\0' ? NULL : display;
}

static const char *group_description(const char *original, const char *group)
{
    const char *path;

    if (gethashnode(aliastab, original) != NULL)
        return "alias";
    if (gethashnode(shfunctab, original) != NULL)
        return "shell function";
    if (gethashnode(builtintab, original) != NULL)
        return "builtin";
    if (gethashnode(reswdtab, original) != NULL)
        return "reserved word";
    if (group == NULL)
        return NULL;
    if (group[0] == '-' && group[strlen(group) - 1] == '-')
        return NULL;
    if (strcmp(group, "commands") == 0) {
        path = findcmd((char *)original, 1, 0);
        return path == NULL ? "command" : path;
    }
    if (strcmp(group, "builtins") == 0)
        return "builtin";
    if (strcmp(group, "functions") == 0)
        return "shell function";
    if (strcmp(group, "aliases") == 0)
        return "alias";
    if (strcmp(group, "suffix-aliases") == 0)
        return "suffix alias";
    if (strcmp(group, "reserved-words") == 0)
        return "reserved word";
    return group;
}

void keel_completion_capture_reset(void)
{
    capture_length = 0;
    capture_count = 0;
    capture_active = 1;
}

void keel_completion_capture_match(char *original, char *display, char *ignored_prefix,
                                   char *prefix, char *path_prefix, char *match,
                                   char *path_suffix, char *suffix,
                                   char *ignored_suffix, int flags, char *group)
{
    const char *detail;
    const char *candidate[] = {
        ignored_prefix, prefix, path_prefix, match, path_suffix, suffix, ignored_suffix,
    };
    size_t start = capture_length;
    size_t index;

    (void)flags;
    if (!capture_active || capture_count == KEEL_COMPLETION_LIMIT || original == NULL ||
        original[0] == '\0' || match == NULL)
        return;
    detail = description_for(original, display);
    if (detail == NULL)
        detail = group_description(original, group);
    if (!append_decoded(original, strlen(original)) || !append_byte(0x1f) ||
        !append_decoded(detail, detail == NULL ? 0 : strlen(detail)) ||
        !append_byte(0x1f)) {
        capture_length = start;
        return;
    }
    for (index = 0; index < sizeof(candidate) / sizeof(candidate[0]); index++) {
        if (!append_decoded(candidate[index],
                            candidate[index] == NULL ? 0 : strlen(candidate[index]))) {
            capture_length = start;
            return;
        }
    }
    if (!append_byte(0x1e)) {
        capture_length = start;
        return;
    }
    capture_count++;
}

static int write_capture(const unsigned char *bytes, size_t length)
{
    while (length > 0) {
        ssize_t written = write(STDOUT_FILENO, bytes, length);
        if (written < 0 && errno == EINTR)
            continue;
        if (written <= 0)
            return 0;
        bytes += written;
        length -= (size_t)written;
    }
    return 1;
}

void keel_completion_capture_finish(const char *request, const char *elapsed)
{
    unsigned char header[96];
    int header_length;

    if (!capture_active || request == NULL || elapsed == NULL)
        return;
    capture_active = 0;
    header_length = snprintf((char *)header, sizeof(header), "K1\x1f%s\x1f%s\x1e",
                             request, elapsed);
    if (header_length <= 0 || (size_t)header_length >= sizeof(header))
        return;
    if (!write_capture(header, (size_t)header_length) ||
        !write_capture(capture_buffer, capture_length))
        return;
    (void)write_capture((const unsigned char *)"\x1d", 1);
}
