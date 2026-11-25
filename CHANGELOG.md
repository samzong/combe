## [0.2.7] (2026-10-18)


### Features

* **menu:** add system About and cargo-release cuts
* **log:** add rotating file logger for diagnostics
* **telemetry:** record a local, default-off behavior study log (#14)


### Fixes

* **cli:** trust the nest marker only from a descendant process (#12)
* **sidebar:** collapse repo group when its last tab closes
* **combe:** report search state changes and refocus on tab close
* **makefile:** strip header dashes from make help in C locale (#15)
* **terminal:** use a portable TERM for ssh sessions (#20)
* **window:** remove the final tab after shell exit (#22)


### Refactors

* **cli:** simplify usage text and drop state path output
* **chrome:** separate ownership and remove duplicate paths (#11)
* **split:** extract rect-based neighbor picker
* **combe:** restrict pub items to crate visibility


### Documentation

* **readme:** embed the introduction video inline


### Maintenance

* **ghostty-sys:** link libghostty without requiring full Xcode
* **package:** omit unused Ghostty themes and docs from the app (#13)
* enable strict workspace lints and fix violations
* **ghostty-sys:** make zig cache configurable via COMBE_ZIG_CACHE
