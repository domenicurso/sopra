#include "internal.h"

#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

#define KEEL_COMPLETION_LIMIT 16384u
#define KEEL_ZSH_META 0x83u
#define KEEL_CMF_FILE (1u << 0)

#ifndef PATH_MAX
#define PATH_MAX 4096
#endif

static unsigned char capture_buffer[KEEL_MAX_HOST_BYTES];
static unsigned char candidate_buffer[KEEL_MAX_HOST_BYTES];
static unsigned char stat_path_buffer[PATH_MAX + 1u];
static size_t capture_length;
static size_t capture_count;
static size_t candidate_length;
static size_t stat_path_length;
static int capture_active;

static int append_byte(unsigned char byte)
{
    if (capture_length == sizeof(capture_buffer))
        return 0;
    capture_buffer[capture_length++] = byte;
    return 1;
}

static int append_bytes_to(unsigned char *target, size_t *target_length, size_t capacity,
                           const unsigned char *value, size_t length)
{
    size_t index;

    if (value == NULL)
        return 1;
    for (index = 0; index < length; index++) {
        unsigned char byte = value[index];

        if (byte == 0 || byte == 0x1d || byte == 0x1e || byte == 0x1f ||
            byte == '\r' || byte == '\n')
            return 0;
        if (*target_length == capacity)
            return 0;
        target[(*target_length)++] = byte;
    }
    return 1;
}

static int append_decoded_to(unsigned char *target, size_t *target_length, size_t capacity,
                             const char *value, size_t length)
{
    size_t index;

    if (value == NULL)
        return 1;
    for (index = 0; index < length; index++) {
        unsigned char byte = (unsigned char)value[index];

        if (byte == KEEL_ZSH_META && index + 1 < length)
            byte = (unsigned char)value[++index] ^ 32u;
        if (!append_bytes_to(target, target_length, capacity, &byte, 1))
            return 0;
    }
    return 1;
}

static int append_decoded(const char *value, size_t length)
{
    return append_decoded_to(capture_buffer, &capture_length, sizeof(capture_buffer), value,
                             length);
}

static int append_candidate_bytes(void)
{
    return append_bytes_to(capture_buffer, &capture_length, sizeof(capture_buffer),
                           candidate_buffer, candidate_length);
}

static int build_candidate(const char *const parts[], size_t part_count)
{
    size_t index;

    candidate_length = 0;
    for (index = 0; index < part_count; index++) {
        const char *part = parts[index];

        if (!append_decoded_to(candidate_buffer, &candidate_length, sizeof(candidate_buffer),
                               part, part == NULL ? 0 : strlen(part)))
            return 0;
    }
    return candidate_length > 0;
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

static int append_function_path(const char *original)
{
    KeelZshFunctionNode *function = gethashnode(shfunctab, original);
    size_t path_length;

    if (function == NULL || function->filename == NULL)
        return 0;
    path_length = strlen(function->filename);
    if (!append_decoded(function->filename, path_length))
        return 0;
    if ((function->node.flags & KEEL_ZSH_PM_LOADDIR) != 0 &&
        !append_byte('/'))
        return 0;
    if ((function->node.flags & KEEL_ZSH_PM_LOADDIR) != 0)
        return append_decoded(original, strlen(original));
    return 1;
}

static const char *path_value_start(const char *value)
{
    const char *equals;

    if (value == NULL)
        return NULL;
    equals = strchr(value, '=');
    if (equals != NULL && value[0] == '-' && strchr(equals, '/') != NULL)
        return equals + 1;
    return value;
}

static int build_stat_path(const char *path_root, const char *original)
{
    const char *path = path_value_start(original);
    size_t root_length;

    stat_path_length = 0;
    if (path == NULL || path[0] == '\0')
        return 0;
    if (path[0] == '/') {
        if (!append_decoded_to(stat_path_buffer, &stat_path_length,
                               sizeof(stat_path_buffer) - 1u, path, strlen(path)))
            return 0;
    } else if (path_root != NULL && path_root[0] != '\0') {
        root_length = strlen(path_root);
        if (!append_decoded_to(stat_path_buffer, &stat_path_length,
                               sizeof(stat_path_buffer) - 1u, path_root, root_length))
            return 0;
        if (stat_path_length > 0 && stat_path_buffer[stat_path_length - 1] != '/' &&
            !append_bytes_to(stat_path_buffer, &stat_path_length,
                             sizeof(stat_path_buffer) - 1u, (const unsigned char *)"/", 1))
            return 0;
        if (!append_decoded_to(stat_path_buffer, &stat_path_length,
                               sizeof(stat_path_buffer) - 1u, path, strlen(path)))
            return 0;
    } else if (path[0] == '~' && path[1] == '/') {
        const char *home = getenv("HOME");

        if (home == NULL ||
            !append_decoded_to(stat_path_buffer, &stat_path_length,
                               sizeof(stat_path_buffer) - 1u, home, strlen(home)) ||
            !append_decoded_to(stat_path_buffer, &stat_path_length,
                               sizeof(stat_path_buffer) - 1u, path + 1, strlen(path + 1)))
            return 0;
    } else if (!append_decoded_to(stat_path_buffer, &stat_path_length,
                                  sizeof(stat_path_buffer) - 1u, path, strlen(path))) {
        return 0;
    }
    stat_path_buffer[stat_path_length] = '\0';
    return stat_path_length > 0;
}

static int stat_completion_path(const char *path_root, const char *original,
                                struct stat *metadata)
{
    if (!build_stat_path(path_root, original))
        return 0;
    if (stat((char *)stat_path_buffer, metadata) == 0)
        return 1;
    return lstat((char *)stat_path_buffer, metadata) == 0;
}

static char path_kind_for(const char *original, int flags, unsigned long mode,
                          unsigned long followed_mode, const struct stat *metadata)
{
    unsigned long effective_mode;

    if ((flags & (int)KEEL_CMF_FILE) == 0)
        return '\0';
    if (original != NULL && original[0] != '\0' && original[strlen(original) - 1] == '/')
        return 'd';
    if (metadata != NULL)
        return S_ISDIR(metadata->st_mode) ? 'd' : 'f';
    effective_mode = followed_mode != 0 ? followed_mode : mode;
    return effective_mode != 0 && S_ISDIR((mode_t)effective_mode) ? 'd' : 'f';
}

static int is_positional_placeholder(const char *value)
{
    size_t index;

    if (value == NULL || value[0] != '$' || value[1] == '\0')
        return 0;
    for (index = 1; value[index] != '\0'; index++) {
        if (value[index] < '0' || value[index] > '9')
            return 0;
    }
    return 1;
}

static int format_relative_mtime(char *output, size_t capacity, time_t modified)
{
    long long delta = (long long)time(NULL) - (long long)modified;
    unsigned long long seconds;
    unsigned long long amount;
    const char *unit;

    if (delta < 0) {
        seconds = (unsigned long long)(-(delta + 1)) + 1u;
        if (seconds < 60u)
            return snprintf(output, capacity, "in under a minute") > 0;
    } else {
        seconds = (unsigned long long)delta;
        if (seconds < 60u)
            return snprintf(output, capacity, "just now") > 0;
    }
    if (seconds < 3600u) {
        amount = seconds / 60u;
        unit = "minute";
    } else if (seconds < 86400u) {
        amount = seconds / 3600u;
        unit = "hour";
    } else if (seconds < 604800u) {
        amount = seconds / 86400u;
        unit = "day";
    } else if (seconds < 2592000u) {
        amount = seconds / 604800u;
        unit = "week";
    } else if (seconds < 31536000u) {
        amount = seconds / 2592000u;
        unit = "month";
    } else {
        amount = seconds / 31536000u;
        unit = "year";
    }
    if (amount == 0)
        amount = 1;
    return delta < 0 ?
               snprintf(output, capacity, "in %llu %s%s", amount, unit,
                        amount == 1 ? "" : "s") > 0 :
               snprintf(output, capacity, "%llu %s%s ago", amount, unit,
                        amount == 1 ? "" : "s") > 0;
}

static int append_group_description(const char *original, const char *group)
{
    char *path;

    if (gethashnode(aliastab, original) != NULL)
        return append_decoded("alias", sizeof("alias") - 1);
    if (gethashnode(shfunctab, original) != NULL) {
        if (append_function_path(original))
            return 1;
        return append_decoded("shell function", sizeof("shell function") - 1);
    }
    if (gethashnode(builtintab, original) != NULL)
        return append_decoded("builtin", sizeof("builtin") - 1);
    if (gethashnode(reswdtab, original) != NULL)
        return append_decoded("reserved word", sizeof("reserved word") - 1);

    /* Command-name providers do not all use the same completion group. */
    path = findcmd((char *)original, 1, 0);
    if (path != NULL)
        return append_decoded(path, strlen(path));

    if (group == NULL)
        return 1;
    if (group[0] == '-' && group[strlen(group) - 1] == '-')
        return 1;
    if (strcmp(group, "commands") == 0) {
        return append_decoded("command", sizeof("command") - 1);
    }
    if (strcmp(group, "builtins") == 0)
        return append_decoded("builtin", sizeof("builtin") - 1);
    if (strcmp(group, "functions") == 0)
        return append_decoded("shell function", sizeof("shell function") - 1);
    if (strcmp(group, "aliases") == 0)
        return append_decoded("alias", sizeof("alias") - 1);
    if (strcmp(group, "suffix-aliases") == 0)
        return append_decoded("suffix alias", sizeof("suffix alias") - 1);
    if (strcmp(group, "reserved-words") == 0)
        return append_decoded("reserved word", sizeof("reserved word") - 1);
    return append_decoded(group, strlen(group));
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
                                   char *ignored_suffix, int flags, char *group,
                                   char *path_root, unsigned long mode,
                                   unsigned long followed_mode)
{
    const char *detail;
    const char *label;
    char kind;
    char relative_mtime[80];
    struct stat metadata;
    int has_metadata = 0;
    const char *candidate[] = {
        ignored_prefix, prefix, path_prefix, match, path_suffix, suffix, ignored_suffix,
    };
    size_t start = capture_length;

    (void)path_root;

    if (!capture_active || capture_count == KEEL_COMPLETION_LIMIT || original == NULL ||
        original[0] == '\0' || match == NULL)
        return;
    if (!build_candidate(candidate, sizeof(candidate) / sizeof(candidate[0])))
        return;
    /* Some generated CLI providers expose an optional positional argument as
     * a shell-style `$0` placeholder. It is metadata about the command's
     * argument, not a completion the user can insert; the path-aware source
     * supplies the real entries once the argument is active. */
    if (is_positional_placeholder(original))
        return;
    kind = path_kind_for(original, flags, mode, followed_mode, NULL);
    if (kind != '\0')
        has_metadata = stat_completion_path(NULL, (char *)candidate_buffer, &metadata);
    /* Approximate Zsh providers can emit the typed fuzzy path as if it were
     * a completion. Only retain path records that resolve to a real entry;
     * the explicit fuzzy resolver supplies the actual matching paths. */
    if (kind != '\0' && !has_metadata)
        return;
    if (kind != '\0' && has_metadata)
        kind = path_kind_for(original, flags, mode, followed_mode, &metadata);
    if ((keel_completion_path_mode == 1 && kind == 'f') ||
        (keel_completion_path_mode == 2 && kind == 'd'))
        return;
    label = kind == '\0' ? original : (char *)candidate_buffer;
    detail = description_for(original, display);
    if (kind != '\0' && has_metadata &&
        !format_relative_mtime(relative_mtime, sizeof(relative_mtime), metadata.st_mtime))
        has_metadata = 0;
    if ((kind == '\0' ? !append_decoded(label, strlen(label)) : !append_candidate_bytes()) ||
        !append_byte(0x1f) ||
        (kind != '\0' && has_metadata ? !append_decoded(relative_mtime,
                                                         strlen(relative_mtime)) :
         (detail != NULL ? !append_decoded(detail, strlen(detail)) :
                           !append_group_description(original, group))) ||
        !append_byte(0x1f)) {
        capture_length = start;
        return;
    }
    if (!append_candidate_bytes() || !append_byte(0x1f) ||
        !append_decoded(kind == 'd' ? "directory" : kind == 'f' ? "file" : "generic",
                        kind == '\0' ? sizeof("generic") - 1 :
                        kind == 'd' ? sizeof("directory") - 1 : sizeof("file") - 1)) {
        capture_length = start;
        return;
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
