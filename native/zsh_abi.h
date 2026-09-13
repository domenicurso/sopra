#ifndef KEEL_ZSH_ABI_H
#define KEEL_ZSH_ABI_H

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <wchar.h>

typedef struct module *Module;
typedef struct widget *Widget;
typedef void *KeelHashTable;
typedef struct {
    void *next;
    char *name;
    int flags;
} KeelZshHashNode;
typedef struct {
    KeelZshHashNode node;
    char *filename;
} KeelZshFunctionNode;
typedef int (*ZleIntFunc)(char **);
typedef void (*KeelRedrawCallback)(void);
typedef void (*KeelCompletionMatchCallback)(
    char *, char *, char *, char *, char *, char *, char *, char *, char *, int,
    char *);

#define KEEL_ZSH_ABI_VERSION 3u
#define KEEL_NATIVE_ABI_VERSION 1u
#define KEEL_MAX_HOST_BYTES (4u * 1024u * 1024u)
#define KEEL_FDT_MODULE 3
#define KEEL_META_DUP 3
#define KEEL_ZSH_PM_LOADDIR (1 << 17)

extern Widget addzlefunction(char *name, ZleIntFunc function, int flags);
extern void deletezlefunction(Widget widget);
extern void *zrealloc(void *pointer, size_t size);
extern int movefd(int fd);
extern int zclose(int fd);
extern void addmodulefd(int fd, int type);
extern void setiparam_no_convert(char *name, long value);
extern void setsparam(char *name, char *value);
extern char *metafy(char *value, int length, int mode);
extern void execstring(char *source, int dont_change_job, int exiting, char *context);
extern char *findcmd(char *name, int copy, int default_path);
extern void *gethashnode(KeelHashTable table, const char *name);

extern KeelHashTable aliastab;
extern KeelHashTable builtintab;
extern KeelHashTable reswdtab;
extern KeelHashTable shfunctab;
extern KeelHashTable sufaliastab;

extern int SHTTY;
extern FILE *shout;
extern int zleactive;
extern int lastval;
extern int zterm_columns;
extern int zterm_lines;
extern int zlecs;
extern int zlell;
extern wchar_t *zleline;
extern void ungetbytes(char *value, int length);
/* Exported by the Keel-enabled Zsh build, immediately after redisplay. */
extern unsigned int keel_zsh_abi_version;
extern KeelRedrawCallback keel_pre_redraw_callback;
extern KeelRedrawCallback keel_post_redraw_callback;
extern KeelCompletionMatchCallback keel_completion_match_callback;
extern uint64_t keel_redisplay_generation;
extern int keel_zle_cursor_column;
extern int keel_zle_cursor_line;
extern int keel_completion_option_mode;

enum {
    KEEL_ZLE_NOTCOMMAND = 1 << 10,
    KEEL_ZLE_NOLAST = 1 << 14,
};

#endif
