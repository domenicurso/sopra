#include "internal.h"

#include <stdio.h>
#include <string.h>

static void paint_fake_cursor_cell(void)
{
    static const unsigned char save_current_cursor[] = "\033[s";
    static const unsigned char full_cursor[] = "\033[7m";
    static const unsigned char cursor_cell[] = " ";
    static const unsigned char restore_current_cursor[] = "\033[u";
    static const unsigned char reset_attributes[] = "\033[0m";
    unsigned char background[3];
    unsigned char color[3];
    char sequence[64];
    int length;

    (void)keel_write_all(save_current_cursor, sizeof(save_current_cursor) - 1);
    if (keel_cursor_blended_color(color) && keel_terminal_background_color(background)) {
        length = snprintf(sequence, sizeof(sequence),
                          "\033[38;2;%u;%u;%um\033[48;2;%u;%u;%um",
                          background[0], background[1], background[2], color[0], color[1],
                          color[2]);
        if (length > 0 && (size_t)length < sizeof(sequence))
            (void)keel_write_all((const unsigned char *)sequence, (size_t)length);
    } else {
        (void)keel_write_all(full_cursor, sizeof(full_cursor) - 1);
    }
    (void)keel_write_all(cursor_cell, sizeof(cursor_cell) - 1);
    (void)keel_write_all(reset_attributes, sizeof(reset_attributes) - 1);
    (void)keel_write_all(restore_current_cursor, sizeof(restore_current_cursor) - 1);
}

void keel_restore_fake_cursor_cell(void)
{
    static const unsigned char save_current_cursor[] = "\033[s";
    static const unsigned char restore_current_cursor[] = "\033[u";
    static const unsigned char reset_attributes[] = "\033[0m";

    if (!keel_fake_cursor_cell_active)
        return;
    if (SHTTY >= 0) {
        (void)keel_write_all(save_current_cursor, sizeof(save_current_cursor) - 1);
        (void)keel_write_all(reset_attributes, sizeof(reset_attributes) - 1);
        (void)keel_write_all(keel_active_cursor_text, keel_active_cursor_text_length);
        (void)keel_write_all(reset_attributes, sizeof(reset_attributes) - 1);
        (void)keel_write_all(restore_current_cursor, sizeof(restore_current_cursor) - 1);
    }
    keel_fake_cursor_cell_active = 0;
}

void keel_write_fake_cursor_cell(void)
{
    if (SHTTY < 0)
        return;
    memcpy(keel_active_cursor_text, keel_fake_cursor_text, keel_fake_cursor_text_length);
    keel_active_cursor_text_length = keel_fake_cursor_text_length;
    paint_fake_cursor_cell();
    keel_fake_cursor_cell_active = 1;
}

void keel_refresh_fake_cursor_cell(void)
{
    if (!keel_fake_cursor_cell_active || SHTTY < 0)
        return;
    paint_fake_cursor_cell();
}

void keel_clear_fake_cursor_cell(void)
{
    keel_restore_fake_cursor_cell();
}
