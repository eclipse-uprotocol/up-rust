Run OpenFastTrace CLI's `trace` command on the local `up-rust` workspace.

Considers all (relevant) specification documents from the `up-spec` submodule and all
implementation and documentation artifacts contained in the `up-rust` repository.

The action installs Temurin JDK 17 for running OpenFastTrace (OFT) and has the following outputs:

| Name                              | Description                                                                                                                                               |
| :-------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `requirements-tracing-exit-code`  | 0: OFT has run successfully and all specification items are covered<br>1: OFT has either failed to run or at least one specification item is not covered. |
| `requirements-tracing-report-url` | The URL pointing to the HTML report that has been created by OFT.                                                                                         |
