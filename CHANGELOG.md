# Changelog

<!-- https://keepachangelog.com/en/1.0.0/ -->

## [0.8.0] - 2025-01-17

### Added

- New languages:

  - OpenSCAD

## [0.7.0] - 2025-11-26

- Bump dependencies, including `tree-sitter`

## [0.6.0] - 2025-03-12

- Fix a bug where the splicer looped infinitely on empty inputs
- Bump dependencies

## [0.5.0] - 2023-07-17

- Small changes to library API
- Bump dependencies

## [0.4.0] - 2023-04-06

- Add `--max-size`
- Add `--reparse`
- Bump dependencies

## [0.3.1] - 2023-03-24

- Fix a panic

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
[0.3.1]: https://github.com/langston-barrett/tree-splicer/releases/tag/v0.3.1
[0.4.0]: https://github.com/langston-barrett/tree-splicer/releases/tag/v0.4.0
[0.5.0]: https://github.com/langston-barrett/tree-splicer/releases/tag/v0.5.0
[0.6.0]: https://github.com/langston-barrett/tree-splicer/releases/tag/v0.6.0
