#include "internal.h"

#include <string.h>

void keel_capture_fake_cursor_text(void)
{
    mbstate_t state;
    size_t length;

    keel_fake_cursor_text[0] = ' ';
    keel_fake_cursor_text_length = 1;
    if (zleline == NULL || zlecs < 0 || zlecs >= zlell)
        return;
    memset(&state, 0, sizeof(state));
    length = wcrtomb((char *)keel_fake_cursor_text, zleline[zlecs], &state);
    if (length == (size_t)-1 || length == 0 || length > sizeof(keel_fake_cursor_text)) {
        keel_fake_cursor_text[0] = ' ';
        keel_fake_cursor_text_length = 1;
        return;
    }
    keel_fake_cursor_text_length = length;
}
