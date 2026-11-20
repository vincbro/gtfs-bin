# Agent notes for flatnet

Before writing or reviewing code here, read:

- [CONTRIBUTING.md](CONTRIBUTING.md): setup, `just` recipes, lints, formatting and commits.
- [docs/FORMAT.md](docs/FORMAT.md): the layout of the compiled file. A change to a table or a header field changes it and bumps `FLATNET_VERSION`.
- [STANDARDS.local.md](STANDARDS.local.md): flatnet's gate, hot paths, known performance problems and the overrides of the shared rules.
- [TODO.md](TODO.md): what is done, open, and decided against.

`just check` is the gate, and it must pass.

## Code standards

Rules live in `code-standards/` (a submodule). Read `code-standards/<language>/` before writing or reviewing code in that language, and `STANDARDS.local.md` for what is specific to this project.
Use the `review` skill to review a change and the `commit` skill to commit. Run `review` on your own changes before you call any code change done.
