set timeout 30
set repo [lindex $argv 0]
set zsh_bin [lindex $argv 1]

if {$repo eq "" || $zsh_bin eq ""} {
    puts stderr "usage: native.exp REPO_ROOT ZSH_BIN"
    exit 2
}

set env(TERM) xterm-256color
set env(LANG) C.UTF-8
set env(LC_ALL) C.UTF-8
set env(LC_CTYPE) C.UTF-8
if {![info exists env(KEEL_MODULE_PATH)]} {
    set env(KEEL_MODULE_PATH) "$repo/target/debug"
}
set startup_dir "/tmp/keel-native-zshrc-[pid]"
file mkdir $startup_dir
set startup_file [open [file join $startup_dir .zshrc] w]
puts $startup_file "PROMPT='native-prompt> '"
puts $startup_file "RPROMPT=''"
puts $startup_file "autoload -Uz compinit"
puts $startup_file "compinit -C"
puts $startup_file "zstyle ':completion:*' verbose yes"
puts $startup_file "zstyle ':completion:*:options' verbose yes"
puts $startup_file {typeset -a _keel_test_values=(alpha alpine beta)}
puts $startup_file {typeset -a _keel_test_descriptions=('first result' 'second result' 'third result')}
puts $startup_file {_keel_test_complete() { compadd -d _keel_test_descriptions -- "${_keel_test_values[@]}"; }}
puts $startup_file {typeset -a _keel_many_values=(item-00 item-01 item-02 item-03 item-04 item-05 item-06 item-07 item-08 item-09 item-10 item-11 item-12 item-13 item-14 item-15 item-16 item-17 item-18 item-19)}
puts $startup_file {typeset -a _keel_many_descriptions=('item zero' 'item one' 'item two' 'item three' 'item four' 'item five' 'item six' 'item seven' 'item eight' 'item nine' 'item ten' 'item eleven' 'item twelve' 'item thirteen' 'item fourteen' 'item fifteen' 'item sixteen' 'item seventeen' 'item eighteen' 'item nineteen')}
puts $startup_file {_keel_many_complete() { compadd -d _keel_many_descriptions -- "${_keel_many_values[@]}"; }}
puts $startup_file {_keel_options_complete() { _arguments '1:subcommand:(run inspect search)' '--verbose[show verbose output]' '--format=[select output format]:format:(text json)'; }}
puts $startup_file {typeset -a _keel_fuzzy_values=(--files-with-matches --files-without-match)}
puts $startup_file {typeset -a _keel_fuzzy_descriptions=('print only matching files' 'print files without matches')}
puts $startup_file {_keel_fuzzy_complete() { compadd -d _keel_fuzzy_descriptions -- "${_keel_fuzzy_values[@]}"; }}
puts $startup_file {_keel_command_path_complete() { compadd -- sh; }}
puts $startup_file {keel-test() { print -r -- "keel-test:$*"; }}
puts $startup_file {keel-path-function() { print -r -- "keel-path-function:$*"; }}
puts $startup_file {keel-command-path() { print -r -- "keel-command-path:$*"; }}
puts $startup_file {keel-many() { print -r -- "keel-many:$*"; }}
puts $startup_file {keel-options() { print -r -- "keel-options:$*"; }}
puts $startup_file {keel-fuzzy() { print -r -- "keel-fuzzy:$*"; }}
puts $startup_file {keel-generated-cli() {
    if [[ $1 == completion && $2 == zsh ]]; then
        print -r -- '#compdef keel-generated-cli'
        print -r -- '_keel_generated_cli_complete() {
            local -a values descriptions
            if [[ $words[CURRENT] == --* ]]; then
                values=(--verbose --format)
                descriptions=("verbose mode" "output format")
            else
                values=(run inspect)
                descriptions=("run command" "inspect command")
            fi
            compadd -d descriptions -- "${values[@]}"
        }'
        print -r -- 'compdef _keel_generated_cli_complete keel-generated-cli'
    else
        print -r -- "keel-generated-cli:$*"
    fi
}}
puts $startup_file {for _keel_index in {000..699}; do alias "keel-generated-${_keel_index}=true"; done}
puts $startup_file "compdef _keel_test_complete keel-test"
puts $startup_file "compdef _keel_many_complete keel-many"
puts $startup_file "compdef _keel_options_complete keel-options"
puts $startup_file "compdef _keel_fuzzy_complete keel-fuzzy"
puts $startup_file "compdef _keel_command_path_complete keel-command-path"
puts $startup_file "compdef _grep grep"
puts $startup_file "source '$repo/zsh/keel.zsh'"
close $startup_file
set env(ZDOTDIR) $startup_dir
if {[info exists env(KEEL_TEST_LOG)]} {
    log_user 1
} else {
    log_user 0
}

proc expect_native_prompt {} {
    expect {
        -re {native-prompt> } {}
        timeout {
            puts stderr "timed out waiting for native prompt"
            exit 1
        }
        eof {
            puts stderr "zsh exited before native prompt"
            exit 1
        }
    }
}

proc expect_popup {} {
    expect {
        -re {(?:[0-9]+)?/[0-9]+; [0-9]+\.[0-9]ms} {}
        timeout {
            puts stderr "autocomplete surface did not render"
            exit 1
        }
        eof {
            puts stderr "zsh exited before autocomplete surface"
            exit 1
        }
    }
}

spawn $zsh_bin -di
expect_native_prompt
