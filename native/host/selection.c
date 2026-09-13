#include "internal.h"

#include <limits.h>
#include <locale.h>
#include <string.h>

int keel_replace_zle_line_with_selection(void)
{
    mbstate_t state;
    const char *source;
    size_t replacement_length;
    size_t wide_length;
    size_t wide_capacity = sizeof(wide_line_buffer) / sizeof(*wide_line_buffer) - 1;

    replacement_length = keel_module_selected_replacement(
        (unsigned char *)line_buffer, sizeof(line_buffer) - 1);
    if (replacement_length == 0 || replacement_length >= sizeof(line_buffer) || zleline == NULL)
        return 1;
    line_buffer[replacement_length] = '\0';

    memset(&state, 0, sizeof(state));
    source = line_buffer;
    wide_length = mbsrtowcs(wide_line_buffer, &source, wide_capacity, &state);
    if (wide_length == (size_t)-1 || source != NULL || wide_length > INT_MAX)
        return 1;
    wide_line_buffer[wide_length] = L'\0';

    zleline = zrealloc(zleline, (wide_length + 1) * sizeof(*zleline));
    wmemcpy(zleline, wide_line_buffer, wide_length + 1);
    zlell = (int)wide_length;
    zlecs = zlell;
    keel_module_suppress_overlay_for_line(
        (const unsigned char *)line_buffer, replacement_length);
    return 0;
}
