#!/usr/bin/env bash
# Credential-bearing network helper, installed root-owned; never run as builder.
set -euo pipefail
[[ $# == 1 && $(id -un) == silkfetch ]] || exit 1
[[ $1 =~ ^actions/[A-Za-z0-9/?=\&_.-]+$ ]] || exit 1
token=$(cat /run/silkboard-fetch/token)
[[ $token =~ ^github_pat_[A-Za-z0-9_]+$ ]] || exit 1
# Secret travels over stdin, never in argv, environment, Git config, or logs.
printf 'header = "Authorization: Bearer %s"\n' "$token" |
    curl --config - --fail --silent --show-error --max-time 60 \
        --proto '=https' --max-filesize 4194304 \
        --header 'Accept: application/vnd.github+json' \
        --header 'X-GitHub-Api-Version: 2022-11-28' \
        "https://api.github.com/repos/breki/silkboard/$1"
