#include "internal.h"

#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

void keel_completion_close_worker(int terminate)
{
    if (keel_completion_worker_fd >= 0) {
        zclose(keel_completion_worker_fd);
        keel_completion_worker_fd = -1;
    }
    if (keel_completion_worker_pid > 0) {
        if (terminate)
            (void)kill(-keel_completion_worker_pid, SIGKILL);
        while (waitpid(keel_completion_worker_pid, NULL, terminate ? 0 : WNOHANG) < 0 &&
               errno == EINTR) {
        }
        keel_completion_worker_pid = -1;
    }
    keel_completion_response_length = 0;
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

int keel_completion_start_worker(char **args)
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
    keel_completion_close_worker(1);
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
    keel_completion_worker_pid = pid;
    keel_completion_worker_fd = read_fd;
    keel_completion_response_length = 0;
    setiparam_no_convert("REPLY", (long)read_fd);
    return 0;
}
