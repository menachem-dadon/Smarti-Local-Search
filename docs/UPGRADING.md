# Windows installation and upgrades

The release identity stays `com.smarti.localsearch`, the product name stays
`Smarti Local Search`, and installation remains per user. Increase the version
in the npm workspace, frontend package/lock, both Rust packages and Tauri config
for each release. The About screen and native diagnostics read package versions.

Run the new installer normally over the existing installation. Tauri restores
its saved install directory; our install hook also honors that directory when
`/D` specifies another one. The installer updates the same uninstall entry.
New installers reject silent installation over a higher installed version.

Before replacement/uninstall, Windows Restart Manager closes the application
and any bundled inference worker holding its executable open. Starting in 0.1.1,
workers belong to a Windows job with kill-on-close, so forced application exit
also terminates its worker tree. The install hook checks fresh Restart Manager
sessions: parent/job shutdown can
report a failure even when both processes have exited. A remaining file user
still blocks replacement. Tauri may create an empty directory requested with a
conflicting `/D`; the application payload stays in its saved installation.
The install hook clears the managed
`resources/inference` runtime directory before copying the new payload, removing
obsolete DLLs. Never store personal files inside the managed resources directory.

Upgrades preserve `data`, `cache`, `location.json`, custom roots and settings.
The SQLite migration adds indexed queue summaries and covering status indices,
and merges new default exclusions once. It preserves custom exclusions and later user removals.
0.1.2 moves legacy per-location exclusions into the global settings list, anchoring
relative rules to their original location. That avoids broadening a formerly
local rule to unrelated locations. All exclusions are edited in Settings >
Exclusions; location menus contain no exclusion editor. A normal launch restores Explorer/autostart integration from saved settings.
Uninstall also keeps index data unless the user explicitly removes it separately.

## Release validation

`scripts/package.ps1` builds the canonical installer, verifies its exact version,
and records executable/installer hashes and the source manifest.
`scripts/install-smoke.ps1` repacks the generated NSIS recipe under a separate
QA product/registry identity. The application and runtime payload are the release
files. Self-tests require a private `SMARTI_SEARCH_DATA_DIR`, bypass single-instance
routing and window-state persistence, and leave the personal installation running.
This tests the installer recipe/payload; it does not execute the canonical
installer against the user's personal uninstall registration.

When `artifacts/upgrade-baseline-0.1.1/qa/nsis-output.exe` is available, the test first
installs the isolated 0.1.1 payload (with the older 0.1.0 baseline as a fallback) and copies a previously validated private
0.1.0 index into it. It checks an orphaned old worker, an upgrade without `/D`,
running-app reinstallation with a conflicting `/D`, stale runtime removal,
silent downgrade rejection, custom settings/root exclusions, existing file IDs
and vectors, real native search/media/PDF behavior, and uninstall retention.
Without that baseline it checks clean installation and running reinstallation.
The baseline recipe/payload must be saved before building the newer release.
Reports are written under `artifacts/release`; QA uses no production shortcuts.

Full clean-machine, signing and interactive installer/UI acceptance remain
separate from this local automated validation.

The optional `-WhileBundling` validation runs against the newly generated NSIS
recipe while canonical LZMA compression finishes. `installed-verification.json`
records its version and executable hash. `verify-installed-payload.py` compares
every installed resource, root runtime DLL and executable with the release inputs.
Final verification imports installed-test results only when the executable and
all recorded resource hashes still match. This does not install over the personal
product identity. Generated Office fixture and native-manifest changes already
present in a developer checkout can be preserved separately from a code commit.

Runtime mappings are explicit and do not overlap: model/host/FFmpeg/licenses
remain under resources, while every VC runtime DLL has one destination next to
the main executable. This prevents directory/file mappings from competing for
the same native source file during bundling.

The native IPC startup state is registered before webviews are created. Commands
received while opening/migrating the index wait on blocking workers until setup
finishes, preserving the loading screen without a `state not managed` failure.
Native validation checks the rendered frontend search field and absence of
bootstrap alerts in addition to the inference engine.
