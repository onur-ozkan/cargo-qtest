# cargo-qtest: interactive test runner for rust projects

`cargo-qtest` is a command-line tool that enhances the testing experience for Rust projects by providing an interactive and flexible way to find & select and run tests using pattern matching.

![capture](https://github.com/onur-ozkan/cargo-qtest/assets/39852038/9274506e-58f1-4676-a387-04240b18048f)

### Usage

Same as `cargo test`. All args/flags can still be used.

### Q: Why?

- In big projects, executing particular tests from their path becomes challenging.
- Running particular tests from different modules at the same time (e.g., `apple::test_fn` and `lemon::test_fn`).
- I just want to run all the tests matches with "x pattern" in their names.
- It's cool.
