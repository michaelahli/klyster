#!/usr/bin/env bash

set -euo pipefail

current_ref="${1:-HEAD}"
previous_tag="$(git describe --tags --abbrev=0 "${current_ref}^" 2>/dev/null || true)"

if [[ -n "${previous_tag}" ]]; then
    range="${previous_tag}..${current_ref}"
else
    range="${current_ref}"
fi

version="$(git describe --tags --exact-match "${current_ref}" 2>/dev/null || git rev-parse --short "${current_ref}")"
date_utc="$(date -u +%Y-%m-%d)"

declare -a features=()
declare -a fixes=()
declare -a docs=()
declare -a ci=()
declare -a chores=()
declare -a other=()

while IFS=$'\t' read -r subject sha; do
    [[ -n "${subject}" ]] || continue

    entry="- ${subject} (${sha})"
    case "${subject}" in
        feat\(*|feat:*) features+=("${entry}") ;;
        fix\(*|fix:*) fixes+=("${entry}") ;;
        docs\(*|docs:*) docs+=("${entry}") ;;
        ci\(*|ci:*) ci+=("${entry}") ;;
        chore\(*|chore:*) chores+=("${entry}") ;;
        *) other+=("${entry}") ;;
    esac
done < <(git log --no-merges --pretty=format:'%s%x09%h' "${range}")

print_section() {
    local title="$1"
    shift
    local entries=("$@")

    if [[ "${#entries[@]}" -eq 0 ]]; then
        return
    fi

    printf '\n## %s\n\n' "${title}"
    printf '%s\n' "${entries[@]}"
}

printf '# %s\n\n' "${version}"
printf 'Released on %s UTC.\n' "${date_utc}"

if [[ -n "${previous_tag}" ]]; then
    printf '\nChanges since `%s`.\n' "${previous_tag}"
else
    printf '\nInitial generated changelog.\n'
fi

print_section "Features" "${features[@]}"
print_section "Fixes" "${fixes[@]}"
print_section "Documentation" "${docs[@]}"
print_section "CI" "${ci[@]}"
print_section "Chores" "${chores[@]}"
print_section "Other Changes" "${other[@]}"
