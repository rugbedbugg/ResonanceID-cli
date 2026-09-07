# Release packaging

All channels are generated from one **existing, published stable GitHub release**.
The checked-in Chocolatey recipe retains its historical metadata revision; the
generator writes new versioned recipes under ignored `out/packaging/` without
changing the historical submission. AUR remains a source-build package.

```sh
mise run test-packaging
mise run prepare-packages -- --tag v1.0.1
```

The Packaging workflow validates on packaging changes, published releases and
manual runs. The release workflow also calls it explicitly after publication,
since events created with GitHub's default workflow token do not start other
workflows. Preparation verifies the Windows asset's release checksum and hashes
the source archive. It generates Chocolatey, Winget and AUR metadata together.

- Chocolatey: pack, install, run `--help`, uninstall and retain the tested nupkg.
- Winget: validate manifests, install locally, run `--help`, uninstall and retain
  the tested manifests. Portable installs do not bundle FFmpeg; it stays optional.
- AUR: generate `.SRCINFO` with makepkg, verify sources, build and run package
  tests in a fresh Arch container, install, run `--help` and uninstall. Retain the
  tested PKGBUILD and `.SRCINFO`.

Use **Actions > Packaging > Run workflow** on `main`, enter the release tag and
select `none`, `chocolatey`, `winget` or `aur`. The default is validation only.
Publication requires all validation jobs to succeed and consumes their artifacts.
It never rebuilds the Chocolatey package or regenerates submitted manifests.
One channel is published per dispatch; channels are serialized to avoid races.

| Secret | Used by | Purpose |
| --- | --- | --- |
| `CHOCOLATEY_API_KEY` | Chocolatey environment | Push the tested nupkg |
| `WINGET_TOKEN` | packaging environment | Push to the maintainer's winget-pkgs fork and open an upstream PR |
| `AUR_SSH_KEY` | packaging environment | SSH key registered to the AUR maintainer |
| `AUR_KNOWN_HOSTS` | packaging environment | Independently verified aur.archlinux.org host-key entry |
| `PACKAGING_GPG_PRIVATE_KEY` | packaging environment | Existing configured signing identity for packaging commits |
| `PACKAGING_GPG_PASSPHRASE` | packaging environment | Passphrase for that signing key, if required |

Configure repository Actions secrets or the indicated environments. Do not put
credentials in manifests or source files. The normal repository GITHUB_TOKEN
cannot write to the separate Winget fork; its dedicated token needs that access
and permission to create the upstream PR. Both Winget and AUR commits are signed
and verified before pushing. No unsigned fallback is used.

AUR publication verifies the remote commit and refuses version downgrades.
Winget publication returns the upstream PR URL and reuses an already-open PR.
Chocolatey moderation and Winget acceptance are external steps: a successful
submission does not mean approval. No API keys or signing secrets are needed
for validation. These credentials were not configured when this workflow was added.
