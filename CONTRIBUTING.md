# Contributing to Morflo

Help make common local image and video conversion easier to understand and
more reliable. Start with a reproducible problem or a focused proposal.

## Report a bug

Use the [bug form](https://github.com/snowyukitty/morflo/issues/new?template=bug_report.yml)
and include the Morflo version, operating system, active engine, expected
result and smallest reproducible steps. Check existing issues first.

Do not upload private media, full paths, account details or unredacted logs.
Prefer a synthetic fixture. Use [private security reporting](SECURITY.md) for
vulnerabilities rather than a public issue.

## Suggest an improvement

Describe the task, who needs it, and the current obstacle. A concrete example
helps more than a request for more formats. See the [roadmap](docs/product/roadmap.md)
and [scope](docs/product/scope.md) before starting a large change.

## Work on code or documentation

Read the repository instructions and [development setup](docs/development.md).
Use pnpm with the checked-in lockfile. Keep a change focused and run
`pnpm verify:quality`; real-engine changes also need `pnpm test:real` and
relevant native evidence. Explain any check that could not run.

Preserve originals, default to collision-safe outputs, and keep filesystem and
process authority in Rust. Never add telemetry, remote runtime assets, engine
binaries or user-controlled shell commands. Test formats against generated
fixtures, not extension assumptions. Keep user-facing and technical text in
English. Useful documentation contributions include clearer instructions,
accessible screenshots and reproducible troubleshooting.

Explain the problem, resulting behavior and actual checks in a pull request.
Contributions are licensed under MIT OR Apache-2.0, as described in
[LICENSING.md](LICENSING.md). Be respectful, specific and constructive.
