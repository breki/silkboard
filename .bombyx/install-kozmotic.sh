#!/usr/bin/env bash
# Install the native Git-status hook prerequisite without changing hook trust.
set -euo pipefail
set +x
umask 077
fail() { echo "Silkboard Kozmotic installation: $*" >&2; exit 1; }
[[ $EUID != 0 && ${BOMBYX_PROJECT:-} == silkboard ]] || fail 'run as the Bombyx silkboard agent user'
case $(uname -m) in
    x86_64)
        arch=x86_64
        archive_sum=06b27318f4a1648346800629aa900d4f70a1c8f7f40d6fc3e5cd8129bf0474df
        binary_sum=05460ec4090ae257786e5f76505fe65fd67dd61905f324ce8c2ca3c525118575 ;;
    aarch64)
        arch=aarch64
        archive_sum=9e33278edb05bb945773b2827dc1d1785303bee123c44e114dad4410eb415c83
        binary_sum=00751c25cfb14bd59acc017d211e3b1e09824b7172fee68916456611f989431d ;;
    *) fail 'x86_64 or aarch64 guest required' ;;
esac
dest=$HOME/.claude/bin/kozmotic
if [[ -e $dest || -L $dest ]]; then
    [[ -f $dest && ! -L $dest && -x $dest ]] || fail 'unexpected existing Kozmotic path; inspect before replacing it'
    printf '%s  %s\n' "$binary_sum" "$dest" | sha256sum --check --status || fail 'existing Kozmotic differs from the pinned binary; inspect before replacing it'
    exit 0
fi
work=$(mktemp -d)
install_temp=
trap 'rm -rf -- "$work"; rm -f -- "$install_temp"' EXIT
package=kozmotic-v2.3.0-$arch-unknown-linux-gnu
curl -fSsL --retry 3 "https://github.com/breki/kozmotic/releases/download/v2.3.0/$package.tar.gz" -o "$work/package.tar.gz"
printf '%s  %s\n' "$archive_sum" "$work/package.tar.gz" | sha256sum --check --status
tar -xzf "$work/package.tar.gz" -C "$work" -- "$package/kozmotic"
[[ -f $work/$package/kozmotic && ! -L $work/$package/kozmotic ]] || fail 'release binary must be a regular file'
printf '%s  %s\n' "$binary_sum" "$work/$package/kozmotic" | sha256sum --check --status
mkdir -p "$HOME/.claude/bin"
install_temp=$(mktemp "$HOME/.claude/bin/.kozmotic.XXXXXX")
install -m 0755 "$work/$package/kozmotic" "$install_temp"
# Atomic, no-overwrite publication; preserve any concurrently created command.
ln -T "$install_temp" "$dest"
