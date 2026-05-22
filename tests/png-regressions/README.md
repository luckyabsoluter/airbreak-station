# PNG Regression Fixtures

This directory contains static LCD PNG baselines for firmware-specific regression checks.

Run the default `air10-vauto` set:

```bash
./scripts/run-png-regressions.sh
```

Refresh baselines after an intentional UI change:

```bash
AIRBREAK_PNG_REGRESSION_MODE=update ./scripts/run-png-regressions.sh
```

Useful selectors:

```bash
AIRBREAK_PNG_REGRESSION_FIRMWARES=all ./scripts/run-png-regressions.sh
AIRBREAK_PNG_REGRESSION_FIRMWARES=firmware-list ./scripts/run-png-regressions.sh
AIRBREAK_PNG_REGRESSION_CASES=custom_about,clinical_menu ./scripts/run-png-regressions.sh
```

Firmware binaries remain private and ignored by git. Missing firmware entries are skipped by default; set
`AIRBREAK_PNG_REGRESSION_REQUIRE_FIRMWARE=1` when a local automation run must fail on missing inputs.
