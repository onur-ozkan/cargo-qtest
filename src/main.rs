use inquire::formatter::MultiOptionFormatter;
use inquire::list_option::ListOption;
use inquire::ui::Attributes;
use inquire::ui::Color;
use inquire::ui::RenderConfig;
use inquire::ui::StyleSheet;
use inquire::ui::Styled;
use inquire::validator::Validation;
use inquire::MultiSelect;
use regex::Regex;
use std::env;
use std::ffi::OsString;
use std::process::exit;
use std::process::Command;
use std::process::Stdio;

/// Returns the cargo binary path from the `CARGO` environment
/// variable; this enables us to execute cargo from the correct
/// toolchain.
fn cargo_bin() -> String {
    env::var("CARGO").unwrap_or_else(|_| String::from("cargo"))
}

/// Cargo wrapper to retrieve test metadata.
fn get_cargo_test_output(
    first_args: &[OsString],
    second_args: &[OsString],
) -> Result<Vec<String>, String> {
    println!("Collecting test files from the project..\n");

    let mut cargo = Command::new(cargo_bin());
    let cargo = cargo
        .arg("test")
        .args(first_args)
        .arg("--quiet")
        .arg("--")
        .args(second_args)
        .args(["--list", "--color", "never", "--format", "terse"]);

    cargo.stderr(Stdio::inherit());

    cargo.envs(std::env::vars_os());

    let output = cargo
        .output()
        .map_err(|e| format!("Reading test metadata failed. {}", e))?;

    if !output.status.success() {
        exit(1);
    }

    String::from_utf8(output.stdout)
        .map(|data| data.lines().map(String::from).collect())
        .map_err(|e| format!("Reading stdout failed. {}", e))
}

/// Filters out the test paths with regex from a vector of strings.
fn filter_test_options(lines: Vec<String>) -> Vec<String> {
    let pattern = Regex::new(r": test$").expect("Invalid regex pattern");

    lines
        .into_iter()
        .filter_map(|s| {
            if pattern.is_match(&s) {
                Some(pattern.replace(&s, "").to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Spawns a multi-select prompt for choosing tests to be returned.
fn spawn_prompt_for_tests(options: &[String]) -> Vec<String> {
    let validator = |a: &[ListOption<&String>]| {
        if a.is_empty() {
            Ok(Validation::Invalid(
                "You must select at least one test.".into(),
            ))
        } else {
            Ok(Validation::Valid)
        }
    };

    let formatter: MultiOptionFormatter<'_, String> = &|_answer| {
        // let tests: String = answer.iter().fold(String::new(), |acc, item| {
        //     acc + &format!("{}\n", item.value)
        // });

        // format!("\nFollowing tests will be executed:{}{}", "\n", tests)
        String::default()
    };

    let render_config = RenderConfig::default_colored()
        .with_prompt_prefix(Styled::new(""))
        .with_highlighted_option_prefix(Styled::new(""))
        .with_scroll_up_prefix(Styled::new("▲").with_fg(Color::DarkYellow))
        .with_scroll_down_prefix(Styled::new("▼").with_fg(Color::DarkYellow));

    // List items
    let stylesheet = StyleSheet::new()
        .with_fg(Color::Grey)
        .with_attr(Attributes::ITALIC);
    let render_config = render_config.with_option(stylesheet);

    // Selected item
    let stylesheet = StyleSheet::new()
        .with_fg(Color::LightGreen)
        .with_attr(Attributes::BOLD);
    let render_config = render_config.with_selected_option(Some(stylesheet));

    // Text input
    let stylesheet = StyleSheet::new().with_fg(Color::LightMagenta);
    let render_config = render_config.with_text_input(stylesheet);

    // Shortcuts
    let stylesheet = StyleSheet::new()
        .with_fg(Color::DarkBlue)
        .with_attr(Attributes::BOLD);
    let render_config = render_config.with_help_message(stylesheet);

    let render_config = render_config.with_selected_checkbox(
        Styled::new("+")
            .with_fg(Color::DarkGreen)
            .with_attr(Attributes::BOLD),
    );

    let render_config = render_config.with_unselected_checkbox(
        Styled::new("-")
            .with_fg(Color::DarkRed)
            .with_attr(Attributes::BOLD),
    );

    match MultiSelect::new(
        "Search and select set of tests you wish to execute:",
        options.to_vec(),
    )
    .with_render_config(render_config)
    .with_validator(validator)
    .with_formatter(formatter)
    .with_page_size(20)
    .with_help_message("↑↓: Navigate | Space: Choose | →: Select All, | ←: Undo All")
    .prompt()
    {
        Ok(t) => t,
        Err(_) => {
            // Most probably this is an interrupt from the user
            exit(1);
        }
    }
}

// Application entrypoint which collects test informations, prompts the user to select tests,
// does some tricky stuff around cargo and executes the requested tests.
fn main() {
    let mut args = std::env::args_os().skip(1).collect::<Vec<OsString>>();

    if let Some(arg) = args.get(0) {
        if arg.to_string_lossy().starts_with('+') {
            args = args[1..].to_vec();
        }
    }

    if let Some(arg) = args.get(0) {
        if arg.to_string_lossy().ends_with("qtest") {
            args = args[1..].to_vec();
        }
    }

    let (first_args, second_args) = if let Some(index) = args.iter().position(|x| x == "--") {
        let (f, s) = args.split_at(index);
        (f, &s[1..])
    } else {
        (&args[..], &[][..])
    };

    let lines = match get_cargo_test_output(first_args, second_args) {
        Ok(t) => t,
        Err(err) => {
            eprintln!("{}", err);
            exit(1);
        }
    };

    let options = filter_test_options(lines);

    let answer = spawn_prompt_for_tests(&options);

    let final_items: Vec<String> = options
        .iter()
        .filter(|&value| !answer.contains(value))
        .cloned()
        .collect();

    let mut cargo = Command::new(cargo_bin());

    cargo.envs(std::env::vars_os());

    cargo
        .arg("test")
        .args(first_args)
        .arg("--")
        .args(second_args);

    // We do this weird shit here since cargo does not support running multiple
    // tests individually.
    for item in final_items {
        cargo.args(["--skip", &item]);
    }

    cargo.arg("--exact");

    let mut output = cargo.spawn().unwrap();
    output.wait().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_test_options() {
        let input = [
            "apple_function: test",
            "apple_function2: test",
            "apple_function3 test",
            "apple_function4: ",
            "apple_function5:test",
            "module_t::demo_module::apple_function: test",
            "module_t::demo_module::apple_function2: test",
            "module_t::demo_module::apple_function3 test",
            "module_t::demo_module::apple_function4: ",
            "module_t::demo_module::apple_functiont:test",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let result = crate::filter_test_options(input);
        assert_eq!(
            result,
            vec![
                "apple_function",
                "apple_function2",
                "module_t::demo_module::apple_function",
                "module_t::demo_module::apple_function2"
            ]
        );
    }

    #[test]
    fn test_cargo_bin_when_cargo_env_variable_set() {
        env::set_var("CARGO", "cargo_from_the_toolchain_path");
        assert_eq!(cargo_bin(), "cargo_from_the_toolchain_path");
    }

    #[test]
    fn test_cargo_bin_with_no_cargo_env_variable() {
        env::remove_var("CARGO");
        assert_eq!(cargo_bin(), "cargo");
    }
}
