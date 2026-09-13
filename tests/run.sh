#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
for script in \
    "$repo_root/install.sh" \
    "$repo_root/scripts/build-keel.sh" \
    "$repo_root/scripts/build-zsh.sh" \
    "$repo_root/scripts/install-keel.sh" \
    "$repo_root/scripts/install/files.sh" \
    "$repo_root/scripts/install/output.sh" \
    "$repo_root/scripts/uninstall-keel.sh" \
    "$repo_root/scripts/start-keel.sh" \
    "$repo_root/uninstall.sh" \
    "$repo_root/tests/run.sh"; do
    bash -n "$script"
done
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
"$repo_root/scripts/build-keel.sh"
"$repo_root/target/keel-zsh/bin/zsh" -dfn "$repo_root/zsh/keel.zsh"
"$repo_root/target/keel-zsh/bin/zsh" -dfc \
    'print $(printf substitution-ok)' | grep -qx substitution-ok
KEEL_MODULE_PATH="$repo_root/target/debug" \
    "$repo_root/target/keel-zsh/bin/zsh" -dfc \
    'module_path=($KEEL_MODULE_PATH ${module_path:-}); zmodload zsh/zle && zmodload keel && zmodload zsh/complete && print keel-loaded' \
    | grep -qx keel-loaded
stock_zsh=${KEEL_STOCK_ZSH:-/bin/zsh}
if [[ -x "$stock_zsh" ]] && KEEL_MODULE_PATH="$repo_root/target/debug" \
    "$stock_zsh" -dfc \
    'module_path=($KEEL_MODULE_PATH ${module_path:-}); zmodload zsh/zle; zmodload keel' \
    >/dev/null 2>&1; then
    echo "stock Zsh unexpectedly accepted the Keel module" >&2
    exit 1
fi
if [[ -x "$stock_zsh" ]]; then
    env -u KEEL_MODULE_PATH "$stock_zsh" -dic \
        "source '$repo_root/zsh/keel.zsh'; print stock-loader-safe" \
        | grep -qx 'stock-loader-safe'
fi
install_root="$repo_root/target/keel-install"
install_output=$(KEEL_INSTALL_PREFIX="$install_root" "$repo_root/install.sh")
printf '%s\n' "$install_output" | grep -q '^Keel installed$'
printf '%s\n' "$install_output" | grep -q '^Building Keel: Compiling native module (1/21): '
printf '%s\n' "$install_output" | grep -q '^Installing Keel: Installing shell loader (1/10): '
printf '%s\n' "$install_output" | grep -Fxq "Launch Keel with $install_root/bin/keel -il."
printf '%s\n' "$install_output" | grep -Fxq 'After it starts, run keel status to confirm the shell is enabled.'
if printf '%s\n' "$install_output" | grep -Eq '^  (prefix|launch|isolated|commands):'; then
    echo "installer summary still uses labeled fields" >&2
    exit 1
fi
grep -q 'ZDOTDIR=' "$install_root/bin/keel"
grep -qx 'keel-install-v1' "$install_root/.keel-install"
grep -q 'source .*share/keel.zsh' "$install_root/etc/zsh/.zshrc"
grep -Fq "PROMPT='%F{green}%n%f in %F{cyan}%~%f %F{yellow}>%f '" \
    "$install_root/etc/zsh/.zshrc"
for shell_file in \
    keel-cache.zsh keel-capture.zsh keel-completion.zsh \
    keel-completion-response.zsh keel-lifecycle.zsh keel-vars.zsh keel-widgets.zsh \
    completion/keel-help-parser.zsh completion/keel-help-provider.zsh; do
    if [[ ! -f "$install_root/share/$shell_file" ]]; then
        echo "installer omitted $shell_file" >&2
        exit 1
    fi
done
KEEL_SOURCE_USER_RC=0 "$install_root/bin/keel" -dic 'keel status; exit' | grep -qx 'keel: enabled'
"$install_root/bin/keel" status | grep -q '^keel: installed at '
"$install_root/bin/keel" help | grep -q '^usage: keel '
if "$install_root/bin/keel" disable >/dev/null 2>&1; then
    echo "external keel disable unexpectedly succeeded" >&2
    exit 1
fi
uninstall_guard_root=$(mktemp -d "$repo_root/target/keel-uninstall-guard.XXXXXX")
if KEEL_INSTALL_PREFIX="$uninstall_guard_root" "$repo_root/uninstall.sh" >/dev/null 2>&1; then
    echo "uninstaller accepted an unmarked directory" >&2
    exit 1
fi
if [[ ! -d "$uninstall_guard_root" ]]; then
    echo "uninstaller removed an unmarked directory" >&2
    exit 1
fi
rmdir "$uninstall_guard_root"
KEEL_INSTALL_PREFIX="$install_root" "$repo_root/uninstall.sh" | grep -qx "Keel removed from $install_root"
if [[ -e "$install_root" ]]; then
    echo "uninstaller left the Keel prefix behind" >&2
    exit 1
fi
expect "$repo_root/tests/native.exp" "$repo_root" "$repo_root/target/keel-zsh/bin/zsh"
