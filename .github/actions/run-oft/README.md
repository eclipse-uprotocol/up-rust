Run OpenFastTrace CLI's `trace` command on the local `up-rust` workspace and uploads the report as a workflow artifact.

Considers all (relevant) specification documents from the `up-spec` submodule and all
implementation and documentation artifacts contained in the `up-rust` repository.

The action uses the [standard OpenFastTrace Action](https://github.com/itsallcode/openfasttrace-github-action) for running OpenFastTrace (OFT).

# Inputs

| Name            | Description                                                                                           |
| :-------------- | :---------------------------------------------------------------------------------------------------- |
| `file-patterns` | A whitespace separated list of glob patterns which specify the files to include in the OFT trace run. |

# Outputs

| Name                 | Description                                                                                                                                                 |
| :------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `oft-exit-code`      | 0: OFT has run successfully and all specification items are covered<br>> 1: OFT has either failed to run or at least one specification item is not covered. |
| `tracing-report-url` | The URL pointing to the HTML report that has been created by OFT.                                                                                           |
