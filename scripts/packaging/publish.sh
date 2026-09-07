#!/usr/bin/env bash
set -euo pipefail
test "$GITHUB_REF" = refs/heads/main
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
source_dir="$PWD/tested"
if [[ "$CHANNEL" == aur ]]; then
    : "${AUR_SSH_KEY:?Configure AUR_SSH_KEY}"
    : "${AUR_KNOWN_HOSTS:?Configure verified AUR_KNOWN_HOSTS}"
    key_file="$RUNNER_TEMP/aur-key"
    hosts_file="$RUNNER_TEMP/aur-hosts"
    trap 'rm -f "$key_file" "$hosts_file"' EXIT
    umask 077
    printf '%s\n' "$AUR_SSH_KEY" > "$key_file"
    printf '%s\n' "$AUR_KNOWN_HOSTS" > "$hosts_file"
    export GIT_SSH_COMMAND="ssh -i $key_file -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$hosts_file"
    git clone ssh://aur@aur.archlinux.org/resonanceid-cli.git submission
    cd submission
    current=$(sed -n 's/^pkgver=//p' PKGBUILD)
    if [[ "$current" != "$VERSION" ]] && [[ "$(printf '%s\n%s\n' "$current" "$VERSION" | sort -V | head -n1)" == "$VERSION" ]]; then
        echo 'Refusing to downgrade the AUR package' >&2
        exit 1
    fi
    cp "$source_dir/PKGBUILD" "$source_dir/.SRCINFO" .
    git add PKGBUILD .SRCINFO
    if git diff --cached --quiet; then echo 'AUR already matches the validated recipe'; exit 0; fi
    git commit -S -m "[Packaging]: Update resonanceid-cli to $VERSION"
    git verify-commit HEAD
    git push origin HEAD:master
    test "$(git ls-remote origin refs/heads/master | cut -f1)" = "$(git rev-parse HEAD)"
elif [[ "$CHANNEL" == winget ]]; then
    : "${GH_TOKEN:?Configure WINGET_TOKEN}"
    branch="packaging/resonanceid-cli-$VERSION"
    existing=$(gh pr list --repo microsoft/winget-pkgs --head "$GITHUB_REPOSITORY_OWNER:$branch" --state open --json url --jq '.[0].url // empty')
    if [[ -n "$existing" ]]; then echo "$existing"; exit 0; fi
    gh auth setup-git
    # Create the fork if missing; do not modify existing branches in the fork.
    gh repo fork microsoft/winget-pkgs --clone=false
    git clone --filter=blob:none --no-checkout "https://github.com/$GITHUB_REPOSITORY_OWNER/winget-pkgs.git" submission
    cd submission
    git remote add upstream https://github.com/microsoft/winget-pkgs.git
    git fetch --depth=1 upstream master
    git sparse-checkout init --cone
    manifest="manifests/r/rugbedbugg/ResonanceID-cli/$VERSION"
    git sparse-checkout set "$manifest"
    git switch -c "$branch" FETCH_HEAD
    mkdir -p "$manifest"
    cp "$source_dir"/*.yaml "$manifest/"
    git add "$manifest"
    if git diff --cached --quiet; then echo 'Winget already contains these manifests'; exit 0; fi
    git commit -S -m "[Packaging]: Add ResonanceID CLI $VERSION to Winget"
    git verify-commit HEAD
    git push origin "$branch"
    gh pr create --repo microsoft/winget-pkgs --head "$GITHUB_REPOSITORY_OWNER:$branch" \
        --title "New version: rugbedbugg.ResonanceID-cli version $VERSION" \
        --body 'Add release manifests validated by the packaging workflow, including a Windows install and CLI smoke test.'
else
    echo 'Unsupported publication channel' >&2
    exit 1
fi
