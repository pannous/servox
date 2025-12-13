  ✅ Servo's SpiderMonkey: ESR 140.x (released June 2025 - current version)
  ✅ Firefox 120+: Supports array.new_data (https://bugzilla.mozilla.org/show_bug.cgi?id=1774840)
  ✅ array.new_data is now implemented for constant expressions in our mozjs fork (WasmInitExpr)

- NEVER git reset hard without creating a backup branch or similar
- memory don't write .md documents
- run servo with '&' and kill after 5 sec. Which should be plenty of time together all the output
- no hardcoding, do general cases
- avoid full builds and use incremental build when possible (should be configured as default)
- SessionStart hook automatically syncs with upstream servo/servo:main via sync-upstream.sh
- mv test-* tests/extensions
- The new binary name is servox
## Binary Library Fixes (macOS)
After each `./mach build`, run `./fix-libs.sh` to fix dynamic library paths.
The issue: Rebuilds create new binaries without rpath to /opt/homebrew/lib.
The script adds the rpath and creates version symlinks automatically.
