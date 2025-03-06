# openrct2-autosplitter

This repository houses the ASL file for the OpenRCT2 autosplitter and
the Rust source that generates it. Pushing to this repository will
rebuild the ASL file used by LiveSplit with all supported OpenRCT2
releases included.

The autosplitter is included with LiveSplit and works for RCT1 and
RCT2. Right-click LiveSplit, choose `Edit Splits`, then click
`Activate` on the autosplitter to enable it.

## Supported releases

Any GitHub release for OpenRCT2 that satisfies all of the following
conditions is supported:
- has a zip file containing `symbols-x64` in the name
- has a `.pdb` file inside that zip with the following symbols
  - `gScreenFlag`
  - `gScenarioCompletedCompanyValue` or `OpenRCT2::_gameState` or `_gameState`
- has a zip file containing `"windows-portable-x64"` in the name
- has a file called `openrct2.exe` inside that zip

## How the AutoSplitter works

### Starting

The timer will automatically start when any scenario is started from
the main menu.

### Splitting

The autosplitter will split for you as soon as you complete a
scenario.

### Resetting

The timer will reset automatically if you quit to menu from an active
scenario.

## How the source generator works

The autosplitter is created from part hand-written code and part
generated code. These two parts are simply concatenated together to
create the final ASL file that LiveSplit sources.

The generator downloads the list of GitHub releases for OpenRCT2,
checks for the conditions listed above, then parses the PDB files to
find the memory offsets to specific symbols. It also hashes the binary
for each release so that the autosplitter can determine which version
it's being used with.
