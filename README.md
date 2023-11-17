# cargo-qtest: an interactive test runner for rust projects

`cargo-qtest` is a command-line tool that enhances the testing experience for Rust projects by providing an interactive and flexible way to find & select and run tests using pattern matching.

![asci](https://github.com/onur-ozkan/cargo-qtest/assets/39852038/1675f44f-cfbc-47a1-92a6-cfb9c3590010)

### Install

To install `cargo-qtest`, run the following command:

```sh
cargo install --locked cargo-qtest
```

Once installed, you can start using it with either `cargo qtest` or `cargo-qtest` in any of your projects.

### Usage

The usage of `cargo qtest` mirrors that of cargo test. All arguments and flags applicable to cargo test can still be used with `cargo qtest`.

### Q: Why?

- Sometimes executing specific tests based on their paths can be challenging (specially when there are too many modules).
- Running particular tests from different modules simultaneously (e.g., `apple::test_fn` and `lemon::test_fn`).
- Selectively running tests matching a specific pattern in their names.
- It's cool.
