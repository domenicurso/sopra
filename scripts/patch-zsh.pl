#!/usr/bin/env perl
use strict;
use warnings;

my $root = shift @ARGV;
die "usage: patch-zsh.pl ZSH_SOURCE_ROOT\n" if !defined($root) || @ARGV;

my $zle_path = "$root/Src/Zle/zle_refresh.c";
my $comp_h_path = "$root/Src/Zle/comp.h";
my $compcore_path = "$root/Src/Zle/compcore.c";

sub read_text {
    my ($path) = @_;
    open my $handle, '<:raw', $path or die "keel: cannot read $path: $!\n";
    local $/;
    my $text = <$handle>;
    close $handle or die "keel: cannot close $path: $!\n";
    return $text;
}

sub write_text {
    my ($path, $text) = @_;
    open my $handle, '>:raw', $path or die "keel: cannot write $path: $!\n";
    print {$handle} $text or die "keel: cannot write $path: $!\n";
    close $handle or die "keel: cannot close $path: $!\n";
}

sub replace_after {
    my ($text, $pattern, $replacement, $label) = @_;
    my $count = () = $text =~ /$pattern/g;
    die "keel: anchor '$label' matched $count times\n" if $count != 1;
    $text =~ s/($pattern)/$1 . $replacement/e;
    return $text;
}

sub replace_before_close {
    my ($text, $pattern, $replacement, $label) = @_;
    my $count = () = $text =~ /$pattern\}/g;
    die "keel: anchor '$label' matched $count times\n" if $count != 1;
    $text =~ s/($pattern)(\})/$1 . $replacement . $2/e;
    return $text;
}

my $zle = read_text($zle_path);
my $comp_h = read_text($comp_h_path);
my $compcore = read_text($compcore_path);
my @symbols = qw(
    keel_zsh_abi_version
    keel_pre_redraw_callback
    keel_post_redraw_callback
    keel_completion_match_callback
    keel_redisplay_generation
    keel_zle_cursor_column
    keel_zle_cursor_line
    keel_completion_option_mode
);
my @present = grep {
    index($zle, $_) >= 0 || index($comp_h, $_) >= 0 || index($compcore, $_) >= 0
} @symbols;
if (@present == @symbols) {
    print "keel: Zsh anchors already applied\n";
    exit 0;
}
die "keel: partial Keel Zsh patch detected: @present\n" if @present;

my $abi = <<'EOF';

/* Keel's native module checks this before using the callback ABI. */
/**/
mod_export unsigned int keel_zsh_abi_version = 3;
/**/

/* Keel observes the completed ZLE redisplay without owning ZLE itself. */
/**/
mod_export void (*keel_pre_redraw_callback)(void) = NULL;
/**/
mod_export void (*keel_post_redraw_callback)(void) = NULL;
/**/
mod_export void *keel_completion_match_callback = NULL;
/**/
mod_export unsigned long long keel_redisplay_generation = 0;
/**/
mod_export int keel_zle_cursor_column = 0;
/**/
mod_export int keel_zle_cursor_line = 0;
/**/
mod_export int keel_completion_option_mode = 0;

EOF

my $redraw_depth = <<'EOF';
static int keel_redraw_depth;
EOF

my $pre_redraw = <<'EOF';

    if (keel_redraw_depth++ == 0 && keel_pre_redraw_callback)
	keel_pre_redraw_callback();

EOF

my $post_redraw = <<'EOF';

    keel_zle_cursor_column = vcs;
    keel_zle_cursor_line = vln;
    if (--keel_redraw_depth == 0) {
	keel_redisplay_generation++;
	if (keel_post_redraw_callback)
	    keel_post_redraw_callback();
    }
EOF

my $completion_types = <<'EOF';
typedef void (*KeelCompletionMatchCallback)(
    char *, char *, char *, char *, char *,
    char *, char *, char *, char *, int, char *);
extern int keel_completion_option_mode;
EOF

my $option_mode = <<'EOF';

        if (keel_completion_option_mode &&
            compcurrent > 0 && compwords && compwords[compcurrent - 1]) {
            zsfree(compwords[compcurrent - 1]);
            compwords[compcurrent - 1] = ztrdup("--");
            zsfree(compprefix);
            compprefix = ztrdup("--");
        }
EOF

my $match_callback = <<'EOF';
            if (keel_completion_match_callback)
		((KeelCompletionMatchCallback)
		 keel_completion_match_callback)(
		    cm->orig, cm->disp, cm->ipre, cm->pre,
		    cm->ppre, cm->str, cm->psuf, cm->suf,
		    cm->isuf, cm->flags, mgroup->name);
EOF

$zle = replace_after(
    $zle,
    qr{mod_export int nlnct;\n},
    $abi,
    'redisplay ABI export',
);
$zle = replace_after(
    $zle,
    qr{[ \t]*winh_alloc[ \t]*=[ \t]*-1;[ \t]*/\*[ \t]*allocates[ \t]+window[ \t]+height[ \t]*\*/[ \t]*\n},
    $redraw_depth,
    'redisplay depth declaration',
);
$zle = replace_after(
    $zle,
    qr{[ \t]*if[ \t]*\([ \t]*inlist[ \t]*\)[ \t]*\n[ \t]*return;[ \t]*\n},
    $pre_redraw,
    'pre-redraw callback',
);
$zle = replace_before_close(
    $zle,
    qr{[ \t]*if[ \t]*\([ \t]*remetafy[ \t]*\)[ \t]*\n[ \t]*metafy_line\(\);[ \t]*\n},
    $post_redraw,
    'post-redraw callback',
);

$comp_h = replace_after(
    $comp_h,
    qr{typedef struct cmgroup \*Cmgroup;\n},
    $completion_types,
    'completion callback type',
);

$compcore = replace_after(
    $compcore,
    qr{[ \t]*compcurrent[ \t]*=[ \t]*\(usea[ \t]*\?[ \t]*\(clwpos[ \t]*\+[ \t]*1[ \t]*-[ \t]*aadd\)[ \t]*:[ \t]*0\);[ \t]*\n},
    $option_mode,
    'option completion mode',
);
$compcore = replace_after(
    $compcore,
    qr{[ \t]*cm->rems[ \t]*=[ \t]*dat->rems;[ \t]*\n[ \t]*cm->remf[ \t]*=[ \t]*dat->remf;[ \t]*\n[ \t]*if[ \t]*\([ \t]*disp[ \t]*\)[ \t]*\n[ \t]*cm->disp[ \t]*=[ \t]*dupstring\(\*disp\);[ \t]*\n},
    $match_callback,
    'completion match callback',
);

write_text($zle_path, $zle);
write_text($comp_h_path, $comp_h);
write_text($compcore_path, $compcore);
print "keel: applied semantic Zsh 5.9 anchors\n";
