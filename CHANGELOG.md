# Changelog

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)

## Unreleased

The only breaking change in this release is that `resource.body("")` now can reference path and query parameters. What it means is that if `{path.<param_name>}` or `{query.<param_name>}` is provided, they will be translated to the values received in the request.

### Added

- Support to query and path parameters.
- Support to using query and path parameters in body response.
- Support to dynamic body.

### Changed

- Renamed some internal variables and methods in Resource.
- Changed how server matches resources.

### Fixed

- Deprecation warnings.


## 1.0.1 (2020-03-12)

### Added

- Dual license as MIT and Apache 2.0 for extended compatibility.

https://rust-lang.github.io/api-guidelines/necessities.html#crate-and-its-dependencies-have-a-permissive-license-c-permissive

### Fixed

- When creating multiple resources with same URI, last resource defined would override previous ones, making it impossible to define multiple HTTP verbs for same URI.


## 1.0.0 (2018-10-14)

Initial release

