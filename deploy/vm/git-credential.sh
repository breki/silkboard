#!/usr/bin/env bash
# Only Git's fixed HTTPS GitHub host receives the restricted downloader token.
set -euo pipefail
[[ $# == 1 && $1 == get && $(id -un) == silkfetch ]] || exit 0
protocol='' host=''
while IFS='=' read -r name value; do
    case $name in protocol) protocol=$value ;; host) host=$value ;; esac
done
[[ $protocol == https && $host == github.com ]] || exit 1
token=$(cat /run/silkboard-fetch/token)
[[ $token =~ ^github_pat_[A-Za-z0-9_]+$ ]] || exit 1
printf 'username=x-access-token\npassword=%s\n' "$token"
