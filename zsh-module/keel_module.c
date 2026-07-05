/*
 * keel_module.c - native zsh module bridge for Keel
 */

#define IMPORTING_MODULE_zshQsmain 1
#include "zsh.mdh"
#undef IMPORTING_MODULE_zshQsmain

#define IMPORTING_MODULE_zshQszle 1
#include "Zle/zle.mdh"
#undef IMPORTING_MODULE_zshQszle

#include "keel_bridge.h"

extern mod_import_variable char *postedit;

static Widget keel_accept_widget;
static Widget keel_edit_widget;

static char *
keel_last_error(void)
{
    const char *message = keel_bridge_last_error_message();
    return (char *)(message ? message : "keel bridge error");
}

static char *
keel_prompt_text(void)
{
    char *prompt = getsparam_u("KEEL_PROMPT_TEXT");

    if (prompt && *prompt)
        return prompt;

    return "keel> ";
}

static char *
keel_transient_prompt_text(void)
{
    char *prompt = getsparam_u("KEEL_TRANSIENT_PROMPT_TEXT");

    if (prompt && *prompt)
        return prompt;

    return "> ";
}

static int
keel_activate_runtime(char *nam)
{
    KeelStatusCode status = keel_bridge_activate_editor();

    if (status == KEEL_STATUS_OK)
        return 0;

    zwarnnam(nam, "%s", keel_last_error());
    return 1;
}

static int
keel_deactivate_runtime(void)
{
    keel_bridge_deactivate_editor();
    return 0;
}

static int
keel_run_command_read(char *nam, const char *prompt, const char *initial_buffer,
                      size_t initial_cursor, char **accepted_out)
{
    KeelCommandReadRequest request;
    KeelStatusCode status = KEEL_STATUS_PANIC;
    char *accepted;

    *accepted_out = NULL;

    request.prompt = prompt;
    request.initial_buffer = initial_buffer;
    request.initial_cursor = initial_cursor;

    accepted = keel_bridge_read_command(&request, &status);
    if (status == KEEL_STATUS_OK) {
        *accepted_out = accepted;
        return 0;
    }

    if (status != KEEL_STATUS_CANCELLED)
        zwarnnam(nam, "%s", keel_last_error());

    return (status == KEEL_STATUS_CANCELLED) ? 130 : 1;
}

static int
keel_finish_widget_command(char *accepted, int accept_line)
{
    char *accepted_copy = ztrdup(accepted);

    setline(accepted, ZSL_COPY | ZSL_TOEND);
    keel_bridge_free_string(accepted);
    if (!accept_line) {
        zsfree(accepted_copy);
        zleentry(ZLE_CMD_REFRESH);
        return 0;
    }

    zsfree(postedit);
    postedit = ztrdup(tricat(keel_transient_prompt_text(), accepted_copy, "\n"));
    zsfree(accepted_copy);
    done = 1;
    return 0;
}

static int
keel_widget_common(char *nam, int accept_line)
{
    char *initial_buffer;
    char *accepted = NULL;
    int status;

    if (!zleactive) {
        zwarnnam(nam, "widget requires an active ZLE session");
        return 1;
    }

    if (keel_activate_runtime(nam))
        return 1;

    initial_buffer = zlelineasstring(zleline, zlell, 0, NULL, NULL, 0);

    status = keel_run_command_read(nam, keel_prompt_text(), initial_buffer,
                                   (size_t)zlecs, &accepted);
    zsfree(initial_buffer);

    if (status != 0) {
        if (status == 130)
            setline("", ZSL_COPY | ZSL_TOEND);
        zleentry(ZLE_CMD_REFRESH);
        return (status == 130) ? 0 : status;
    }

    return keel_finish_widget_command(accepted, accept_line);
}

/**/
static int
bin_keel_activate(char *nam, UNUSED(char **args), UNUSED(Options ops),
                  UNUSED(int func))
{
    return keel_activate_runtime(nam);
}

/**/
static int
bin_keel_deactivate(UNUSED(char *nam), UNUSED(char **args), UNUSED(Options ops),
                    UNUSED(int func))
{
    return keel_deactivate_runtime();
}

/**/
static int
bin_keel_command_read(char *nam, char **args, UNUSED(Options ops),
                      UNUSED(int func))
{
    char *accepted = NULL;
    char *initial_buffer = args[0] ? args[0] : "";
    size_t initial_cursor = (size_t)MB_METASTRLEN(initial_buffer);
    int status;

    if (keel_activate_runtime(nam))
        return 1;

    status = keel_run_command_read(nam, keel_prompt_text(), initial_buffer,
                                   initial_cursor, &accepted);
    if (status == 130)
        return 130;
    if (status != 0)
        return status;

    if (accepted) {
        setsparam("REPLY", ztrdup(accepted));
        keel_bridge_free_string(accepted);
    }

    return 0;
}

/**/
static int
keel_accept_line_widget(char **args)
{
    return keel_widget_common(args && args[0] ? args[0] : "keel-accept-line", 1);
}

/**/
static int
keel_edit_line_widget(char **args)
{
    return keel_widget_common(args && args[0] ? args[0] : "keel-edit-line", 0);
}

static struct builtin bintab[] = {
    BUILTIN("keel-activate", 0, bin_keel_activate, 0, 0, 0, NULL, NULL),
    BUILTIN("keel-command-read", 0, bin_keel_command_read, 0, 1, 0, NULL, NULL),
    BUILTIN("keel-deactivate", 0, bin_keel_deactivate, 0, 0, 0, NULL, NULL),
};

static struct features module_features = {
    bintab, sizeof(bintab) / sizeof(*bintab),
    NULL, 0,
    NULL, 0,
    NULL, 0,
    0
};

/**/
int
setup_(UNUSED(Module m))
{
    KeelStatusCode status = keel_bridge_module_init();

    if (status == KEEL_STATUS_OK)
        return 0;

    zwarnnam("keel", "%s", keel_last_error());
    return 1;
}

/**/
int
features_(Module m, char ***features)
{
    *features = featuresarray(m, &module_features);
    return 0;
}

/**/
int
enables_(Module m, int **enables)
{
    return handlefeatures(m, &module_features, enables);
}

/**/
int
boot_(UNUSED(Module m))
{
    keel_accept_widget = addzlefunction("keel-accept-line",
                                        keel_accept_line_widget, 0);
    keel_edit_widget = addzlefunction("keel-edit-line",
                                      keel_edit_line_widget, 0);

    if (!keel_accept_widget || !keel_edit_widget) {
        if (keel_accept_widget)
            deletezlefunction(keel_accept_widget);
        if (keel_edit_widget)
            deletezlefunction(keel_edit_widget);
        keel_accept_widget = NULL;
        keel_edit_widget = NULL;
        zwarnnam("keel", "failed to register Keel ZLE widgets");
        return 1;
    }

    return 0;
}

/**/
int
cleanup_(Module m)
{
    keel_bridge_deactivate_editor();

    if (keel_accept_widget) {
        deletezlefunction(keel_accept_widget);
        keel_accept_widget = NULL;
    }
    if (keel_edit_widget) {
        deletezlefunction(keel_edit_widget);
        keel_edit_widget = NULL;
    }

    return setfeatureenables(m, &module_features, NULL);
}

/**/
int
finish_(UNUSED(Module m))
{
    keel_bridge_module_shutdown();
    return 0;
}
