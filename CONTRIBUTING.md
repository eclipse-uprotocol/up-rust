# Contributing to Eclipse uProtocol

Thanks for your interest in this project. Contributions are welcome!

## Developer resources

Information regarding source code management, builds, coding standards, and
more.

<https://projects.eclipse.org/proposals/eclipse-uprotocol>

The project maintains the following source code repositories

<https://github.com/eclipse-uprotocol>

## Eclipse Contributor Agreement

Before your contribution can be accepted by the project team contributors must
electronically sign the [Eclipse Contributor Agreement](https://www.eclipse.org/legal/eca/) (ECA).

Commits that are provided by non-committers must have a Signed-off-by field in
the footer indicating that the author is aware of the terms by which the
contribution has been provided to the project. The non-committer must
additionally have an Eclipse Foundation account and must have a signed Eclipse
Contributor Agreement (ECA) on file.

For more information, please see the [Eclipse Committer Handbook](https://www.eclipse.org/projects/handbook/#resources-commit).

## Setting up a development environment

You can use any development environment you like to contribute to the uProtocol Rust SDK. However, it is mandatory to use the Rust linter ('[clippy](<https://github.com/rust-lang/rust-clippy>)') for any pull requests you do.
To set up VSCode to run clippy per default every time you save your code, have a look here: [How to use Clippy in VS Code with rust-analyzer?](https://users.rust-lang.org/t/how-to-use-clippy-in-vs-code-with-rust-analyzer/41881)

Similarly, the project requests that markdown is formatted and linted properly - to help with this, it is recommended to use [markdown linters](https://marketplace.visualstudio.com/items?itemName=DavidAnson.vscode-markdownlint).

During development, before submitting a PR, you can use `./tools/fmt_clippy_doc.sh` to run these checks on the workspace.

## Performance evidence

Performance reports preserve raw observations exactly. Integer bencher output
may round a sub-nanosecond observation to `0 ns/iter`; that is a statement about
timer and output resolution, never evidence that an operation is free, absent,
zero-copy, or end-to-end no-copy. Generated prose that contains a zero value must
carry this caveat.

The first measured corpus for a new operation or profile is a baseline, not a
regression or improvement result. Before measuring a candidate, record the
baseline identity, comparison policy, controls, sample/iteration rules,
thresholds, variance treatment, and failure handling. A later comparison may use
only a policy that predates the candidate evidence. Do not retrofit thresholds
after observing the result or compare unrelated machines/toolchains without an
explicitly labeled diagnostic purpose.

There also exists a helper script in ./tools to generate test results and test code coverage reports. These reports are placed in the `./target/tarpaulin` directory. If you use VSCode with the [Coverage Gutters](https://marketplace.visualstudio.com/items?itemName=ryanluker.vscode-coverage-gutters) extension, you can enable display of code coverage information with these settings:

``` json
  "coverage-gutters.coverageBaseDir": "**",
  "coverage-gutters.coverageFileNames": [
    "target/tarpaulin/lcov.info",
  ],
```

## DevContainer

All of these prerequisites are made available as a VSCode devcontainer, configured at the usual place (`.devcontainer`).

## Contact

Contact the project developers via the project's "dev" list.

<https://accounts.eclipse.org/mailing-list/uprotocol-dev>
