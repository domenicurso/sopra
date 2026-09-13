#include "../keel_zsh_module_internal.h"

KeelRuntime runtime;
unsigned char patch_buffer[1024 * 1024];
char line_buffer[256 * 1024];
wchar_t wide_line_buffer[sizeof(line_buffer)];
char cwd_buffer[4096];
const char keymap_buffer[] = "main";
