#ifndef KEEL_ZSH_MODULE_INTERNAL_H
#define KEEL_ZSH_MODULE_INTERNAL_H

#include "zsh_abi.h"

#include <stddef.h>
#include <stdint.h>

extern void keel_module_init(void);
extern void keel_module_shutdown(void);
extern size_t keel_module_before_redraw(unsigned char *output, size_t capacity);
extern size_t keel_module_after_redraw(const void *snapshot, unsigned char *output,
                                       size_t capacity);
extern uint16_t keel_module_last_scroll_rows(void);
extern size_t keel_module_line_finish(unsigned char *output, size_t capacity);
extern int keel_module_has_suggestions(void);
extern int keel_module_set_suggestions(const unsigned char *payload, size_t length,
                                       uint64_t completion_tenths_ms);
extern int keel_module_refresh_suggestions(uint64_t completion_tenths_ms);
extern int keel_module_move_selection(int delta);
extern size_t keel_module_selected_replacement(unsigned char *output, size_t capacity);
extern int keel_module_dismiss_overlay(void);
extern void keel_module_suppress_overlay_for_line(const unsigned char *line, size_t length);

typedef struct {
    uint32_t abi_version;
    const unsigned char *buffer;
    size_t buffer_len;
    size_t cursor_units;
    uint16_t terminal_columns;
    uint16_t terminal_rows;
    uint16_t cursor_column;
    uint16_t cursor_row;
    const unsigned char *cwd;
    size_t cwd_len;
    const unsigned char *keymap;
    size_t keymap_len;
    int32_t last_status;
    uint64_t redisplay_generation;
} KeelNativeHostSnapshot;

extern int keel_module_observe(const KeelNativeHostSnapshot *snapshot);

typedef struct {
    int active;
    int in_callback;
    int line_active;
} KeelRuntime;

extern Widget line_init_widget;
extern Widget line_finish_widget;
extern Widget select_previous_widget;
extern Widget select_next_widget;
extern Widget accept_widget;
extern Widget dismiss_widget;
extern Widget clear_line_widget;
extern Widget set_suggestions_widget;
extern Widget refresh_suggestions_widget;
extern Widget cursor_start_widget;
extern Widget cursor_read_widget;
extern Widget cursor_stop_widget;
extern KeelRuntime runtime;
extern unsigned char patch_buffer[1024 * 1024];
extern char line_buffer[256 * 1024];
extern wchar_t wide_line_buffer[sizeof(line_buffer)];
extern char cwd_buffer[4096];
extern const char keymap_buffer[];

void keel_before_redraw(void);
void keel_after_redraw(void);
void keel_observe_current_line(void);
void keel_set_zle_line_origin(void);
void keel_adjust_zle_line_origin(unsigned int rows);
void keel_write_rust_payload(size_t length);
void keel_write_cursor_style(int block);
void keel_query_terminal_colors(void);
void keel_clear_fake_cursor_cell(void);
void keel_reset_cursor_animation(void);
int keel_start_cursor_animation(void);
void keel_stop_cursor_animation(void);
void keel_cursor_animation_tick(void);
int keel_replace_zle_line_with_selection(void);

int keel_register_widgets(void);
void keel_delete_widgets(void);
int keel_register_completion_widgets(void);
void keel_delete_completion_widgets(void);
void keel_completion_worker_shutdown(void);
void keel_completion_capture_reset(void);
void keel_completion_capture_finish(const char *request, const char *elapsed);
void keel_completion_capture_match(char *original, char *display, char *ignored_prefix,
                                   char *prefix, char *path_prefix, char *match,
                                   char *path_suffix, char *suffix,
                                   char *ignored_suffix, int flags, char *group);

#endif
