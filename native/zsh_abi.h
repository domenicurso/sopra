#ifndef KEEL_ZSH_ABI_H
#define KEEL_ZSH_ABI_H

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <wchar.h>

typedef struct module *Module;
typedef struct widget *Widget;
typedef int (*ZleIntFunc)(char **);
typedef void (*KeelRedrawCallback)(void);

extern Widget addzlefunction(char *name, ZleIntFunc function, int flags);
extern void deletezlefunction(Widget widget);

extern int SHTTY;
extern FILE *shout;
extern int zleactive;
extern int lastval;
extern int zterm_columns;
extern int zterm_lines;
extern int zlecs;
extern int zlell;
extern wchar_t *zleline;

/* Exported by the Keel-enabled Zsh build, immediately after redisplay. */
extern KeelRedrawCallback keel_pre_redraw_callback;
extern KeelRedrawCallback keel_post_redraw_callback;
extern uint64_t keel_redisplay_generation;
extern int keel_zle_cursor_column;
extern int keel_zle_cursor_line;

enum {
    KEEL_ZLE_NOTCOMMAND = 1 << 10,
    KEEL_ZLE_NOLAST = 1 << 14,
};

#endif
