#include "internal.h"

unsigned char keel_fake_cursor_text[MB_LEN_MAX] = {' '};
size_t keel_fake_cursor_text_length = 1;
unsigned char keel_active_cursor_text[MB_LEN_MAX] = {' '};
size_t keel_active_cursor_text_length = 1;
int keel_fake_cursor_cell_active;
