#include "keel_zsh_module_internal.h"

#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

static Widget worker_start_widget;
static Widget worker_read_widget;
static Widget worker_stop_widget;
static Widget capture_finish_widget;
static pid_t worker_pid = -1;
static int worker_fd = -1;
static unsigned char response[KEEL_MAX_HOST_BYTES + 128u];
static size_t response_length;

static void close_worker(int terminate)
{
    if (worker_fd >= 0) {
        zclose(worker_fd);
        worker_fd = -1;
    }
    if (worker_pid > 0) {
        if (terminate)
            (void)kill(-worker_pid, SIGKILL);
        while (waitpid(worker_pid, NULL, terminate ? 0 : WNOHANG) < 0 && errno == EINTR) {
        }
        worker_pid = -1;
    }
    response_length = 0;
}

static int valid_request(const char *request)
{
    const unsigned char *cursor = (const unsigned char *)request;

    if (cursor == NULL || *cursor == '\0')
        return 0;
    while (*cursor != '\0') {
        if (*cursor < '0' || *cursor > '9')
            return 0;
        cursor++;
    }
    return 1;
}

static int valid_option_mode(const char *mode)
{
    return mode == NULL ||
           ((mode[0] == '0' || mode[0] == '1') && mode[1] == '\0');
}

static int silence_child_terminal(void)
{
    int null_fd;

    if (SHTTY == STDOUT_FILENO) {
        SHTTY = -1;
        return 0;
    }
    if (SHTTY < 0)
        return 0;
    null_fd = open("/dev/null", O_WRONLY);
    if (null_fd < 0)
        return -1;
    if (dup2(null_fd, SHTTY) < 0 || dup2(null_fd, STDERR_FILENO) < 0) {
        if (null_fd != SHTTY && null_fd != STDERR_FILENO)
            close(null_fd);
        return -1;
    }
    if (null_fd != SHTTY && null_fd != STDERR_FILENO)
        close(null_fd);
    return 0;
}

static void run_child(int read_fd, int write_fd, const char *request)
{
    char source[96];

    (void)setpgid(0, 0);
    close(read_fd);
    if (dup2(write_fd, STDOUT_FILENO) < 0)
        _exit(1);
    close(write_fd);
    if (silence_child_terminal() < 0)
        _exit(1);
    keel_completion_capture_reset();
    (void)snprintf(source, sizeof(source), "_keel_completion_capture_sync %s", request);
    execstring(source, 1, 0, "keel-completion");
    _exit(0);
}

static int start_worker(char **args)
{
    int pipes[2];
    int read_fd;
    int write_fd;
    int flags;
    int long_option_mode;
    pid_t pid;

    if (args == NULL || !valid_request(args[0]) || !valid_option_mode(args[1]))
        return 1;
    long_option_mode = args[1] != NULL && args[1][0] == '1';
    close_worker(1);
    keel_completion_option_mode = long_option_mode;
    if (pipe(pipes) < 0)
        return 1;
    read_fd = movefd(pipes[0]);
    write_fd = movefd(pipes[1]);
    if (read_fd < 0 || write_fd < 0) {
        if (read_fd >= 0)
            zclose(read_fd);
        if (write_fd >= 0)
            zclose(write_fd);
        return 1;
    }
    pid = fork();
    if (pid == 0)
        run_child(read_fd, write_fd, args[0]);
    if (pid < 0) {
        keel_completion_option_mode = 0;
        zclose(read_fd);
        zclose(write_fd);
        return 1;
    }
    keel_completion_option_mode = 0;
    (void)setpgid(pid, pid);
    zclose(write_fd);
    flags = fcntl(read_fd, F_GETFL, 0);
    if (flags >= 0)
        (void)fcntl(read_fd, F_SETFL, flags | O_NONBLOCK);
    addmodulefd(read_fd, KEEL_FDT_MODULE);
    worker_pid = pid;
    worker_fd = read_fd;
    response_length = 0;
    setiparam_no_convert("REPLY", (long)read_fd);
    return 0;
}

static int read_worker(char **args)
{
    int reached_eof = 0;

    if ((args != NULL && args[0] != NULL) || worker_fd < 0)
        return 2;
    while (response_length < sizeof(response)) {
        ssize_t count = read(worker_fd, response + response_length,
                             sizeof(response) - response_length);
        if (count > 0) {
            response_length += (size_t)count;
            if (memchr(response, 0x1d, response_length) != NULL)
                break;
            continue;
        }
        if (count == 0)
            reached_eof = 1;
        if (count < 0 && errno == EINTR)
            continue;
        if (count < 0 && (errno == EAGAIN || errno == EWOULDBLOCK))
            break;
        break;
    }
    if (memchr(response, 0x1d, response_length) == NULL)
        return reached_eof || response_length == sizeof(response) ? 2 : 1;
    setsparam("REPLY", metafy((char *)response, (int)response_length, KEEL_META_DUP));
    return 0;
}

static int stop_worker(char **args)
{
    if (args != NULL && args[0] != NULL)
        return 1;
    close_worker(1);
    keel_completion_option_mode = 0;
    return 0;
}

static int finish_capture(char **args)
{
    if (args == NULL || args[0] == NULL || args[1] == NULL || args[2] != NULL)
        return 1;
    keel_completion_capture_finish(args[0], args[1]);
    return 0;
}

void keel_completion_worker_shutdown(void)
{
    close_worker(1);
    keel_completion_option_mode = 0;
}

void keel_delete_completion_widgets(void)
{
    Widget *widgets[] = {&capture_finish_widget, &worker_stop_widget,
                         &worker_read_widget, &worker_start_widget};
    size_t index;

    for (index = 0; index < sizeof(widgets) / sizeof(widgets[0]); index++) {
        if (*widgets[index] != NULL) {
            deletezlefunction(*widgets[index]);
            *widgets[index] = NULL;
        }
    }
}

int keel_register_completion_widgets(void)
{
    int flags = KEEL_ZLE_NOTCOMMAND | KEEL_ZLE_NOLAST;

    worker_start_widget = addzlefunction("keel-native-start-completion", start_worker, flags);
    worker_read_widget = addzlefunction("keel-native-read-completion", read_worker, flags);
    worker_stop_widget = addzlefunction("keel-native-stop-completion", stop_worker, flags);
    capture_finish_widget = addzlefunction("keel-native-finish-capture", finish_capture, flags);
    return worker_start_widget != NULL && worker_read_widget != NULL &&
           worker_stop_widget != NULL && capture_finish_widget != NULL;
}
