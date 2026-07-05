#ifndef KEEL_BRIDGE_H
#define KEEL_BRIDGE_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum KeelStatusCode {
    KEEL_STATUS_OK = 0,
    KEEL_STATUS_ALREADY_INITIALIZED = 1,
    KEEL_STATUS_NOT_INITIALIZED = 2,
    KEEL_STATUS_NOT_ACTIVE = 3,
    KEEL_STATUS_INVALID_UTF8 = 4,
    KEEL_STATUS_INVALID_REQUEST = 5,
    KEEL_STATUS_CANCELLED = 6,
    KEEL_STATUS_IO_ERROR = 7,
    KEEL_STATUS_PANIC = 8,
} KeelStatusCode;

typedef struct KeelCommandReadRequest {
    const char *prompt;
    const char *initial_buffer;
    size_t initial_cursor;
} KeelCommandReadRequest;

KeelStatusCode keel_bridge_module_init(void);
void keel_bridge_module_shutdown(void);
KeelStatusCode keel_bridge_activate_editor(void);
void keel_bridge_deactivate_editor(void);
char *keel_bridge_read_command(const KeelCommandReadRequest *request, KeelStatusCode *status_out);
const char *keel_bridge_last_error_message(void);
void keel_bridge_free_string(char *value);

#ifdef __cplusplus
}
#endif

#endif
