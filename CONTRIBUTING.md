# Contributing

## Git Commit

| 类型         | 说明                                                          |
| :----------- | :------------------------------------------------------------ |
| **feat**     | 引入新功能                                                    |
| **fix**      | 修复 Bug                                                      |
| **docs**     | 仅修改文档（如 README, API 文档等）                           |
| **style**    | 代码格式调整（不影响逻辑，如空格、分号、缩进等）              |
| **refactor** | 代码重构（既不修复错误也不添加功能的更改）                    |
| **perf**     | 提高性能的代码更改                                            |
| **test**     | 添加缺失的测试或更正现有的测试                                |
| **workflow** | 工作流相关的变更                                              |
| **build**    | 影响构建系统或外部依赖的更改                                  |
| **ci**       | 持续集成相关的配置文件或脚本更改（如 GitHub Actions, Travis） |
| **chore**    | 其他不修改源代码或测试文件的辅助变更                          |
| **wip**      | 正在开发中（Work in Progress），尚未完成的任务                |

## Testing

We use the standard Rust testing harness, along with assert macros from [`googletest-rust`]:

```shell
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p emmylua_parser
```

If you're unfamiliar with `googletest-rust`, here's a quick overview:

- Use `googletest::prelude::*` in your test modules, and annotate test functions with `#[gtest]`.

- `assert_that!` checks a condition and panics on error:

  ```rust
  assert_that!(2 * 2, eq(4));
  ```

  Prefer `assert_that!(x, eq(y))` to `assert_eq` because the former generates a nice diff for you.

- `expect_that!` checks a condition, marks test as failed on error, and continues execution.

  This is useful when adding multiple test cases to a single `#[gtest]` function:

  ```rust
  // Both expectations will be evaluated and reported when test fails:
  expect_that!(2 * 2, ne(4));
  expect_that!(3 * 3, ne(9));
  ```

- `verify_that!` checks a condition and returns a `googletest::Result`.

- `OrFail::or_fail` converts any `Optional` and `Result` to a `googletest::Result`. It also adds current location
  to an error message. We have a wrapper around it called [`check!`].

## Code style and formatting

We use [`rustfmt`] and [`pre-commit`] to manage project's code style.

- `rustfmt` formats Rust code. Simply run `cargo fmt --all` to reformat all files.

- `pre-commit` fixes common issues like trailing whitespaces or broken symlinks in all text files.

  To run it,

  1. install [`pre-commit`][pre-commit-install],
  2. invoke `pre-commit run --all`.

  If it suits your workflow, you can configure PreCommit to run before every commit. To do so, run `pre-commit install`.
  Note that this is not required because our CI will detect any issues.

[`googletest-rust`]: https://github.com/google/googletest-rust/
[`rustfmt`]: https://rust-lang.github.io/rustfmt/
[`pre-commit`]: https://pre-commit.com/#install
[pre-commit-install]: https://pre-commit.com/#install
[`check!`]: https://github.com/search?q=repo%3AEmmyLuaLs%2Femmylua-analyzer-rust%20macro_rules!%20check&type=code
