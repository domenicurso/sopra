#ifndef KEEL_CURSOR_INTERNAL_H
#define KEEL_CURSOR_INTERNAL_H

#include "../host/internal.h"

#include <limits.h>

extern unsigned char keel_fake_cursor_text[MB_LEN_MAX];
extern size_t keel_fake_cursor_text_length;
extern unsigned char keel_active_cursor_text[MB_LEN_MAX];
extern size_t keel_active_cursor_text_length;
extern int keel_fake_cursor_cell_active;

void keel_capture_fake_cursor_text(void);
void keel_write_fake_cursor_cell(void);
void keel_restore_fake_cursor_cell(void);
void keel_query_terminal_colors(void);
int keel_cursor_blended_color(unsigned char color[3]);
int keel_terminal_background_color(unsigned char color[3]);
void keel_restore_terminal_cursor_color(void);
double keel_cursor_opacity(void);
void keel_reset_cursor_animation(void);
int keel_start_cursor_animation(void);
void keel_stop_cursor_animation(void);
void keel_cursor_animation_tick(void);

#endif
