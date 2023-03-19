# Changelog

<!-- https://keepachangelog.com/en/1.0.0/ -->

## [0.3.0] - 2023-03-19

### Added

- "Chaotic" mutations that may introduce syntax errors
- Deletions of optional nodes, in addition to splices

### Changed

- tree-splicer now re-parses the file after each splice. This means that
  splices can be "stacked", i.e., a subtree can be spliced into a subtree that
  was spliced into the original.

### Fixed

- Fixed a panic

## [0.2.0] - 2023-03-13

### Added

- New languages:

  - JavaScript
  - TypeScript

### Changed

- Apply a random number of mutations per test up to `--mutations`
- Ignore parse errors by default
- Removed a chatty print statement

### Fixed

- `--seed` now works as intended

## [0.1.0] - 2023-03-12

Initial release!

[0.1.0]: https://github.com/langston-barrett/tree-splicer/releases/tag/v0.1.0
[0.2.0]: https://github.com/langston-barrett/tree-splicer/releases/tag/v0.2.0
[0.3.0]: https://github.com/langston-barrett/tree-splicer/releases/tag/v0.3.0