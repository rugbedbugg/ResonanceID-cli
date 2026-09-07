# Repository standards baseline

Adapted from ReAgent's `.github/STANDARDS.md`. Apply this structure to maintained
GitHub repositories, preserving their runtime and dependency version policies.

| Area | Implementation |
| --- | --- |
| Required CI | PRs, main/ci branch pushes, manual and reusable workflow entry |
| Tasks/toolchain | mise tasks; rustup stable; committed Cargo.lock with --locked |
| Validation | Blocking rustfmt, build, Clippy and workspace tests on Linux/Windows |
| Permissions | Read-only defaults, contents write only for publication |
| Scheduling | Cancel superseded CI; do not cancel active releases |
| Optional CD | Tag-version validation, shared CI gate, release-binary smoke checks |
| Artifact integrity | Build checksums verified before and after publication |
| README | Purpose, install, usage, configuration, quick demo, tests, license and CI link |

A checksum manifest verifies integrity, not signed provenance. Hosted protection
settings are separate from workflow configuration. Keep package-metadata revisions
separate from application versions. Validate workflow edits with actionlint and
run the applicable mise tasks and GitHub checks before merging.
