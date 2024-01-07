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
use std::io::BufRead;
use std::io::BufReader;
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
    let mut cargo = Command::new(cargo_bin());
    let cargo = cargo
        .stdout(Stdio::piped())
        .arg("test")
        .args(first_args)
        .arg("--")
        .args(["--list", "--format", "terse"])
        .args(second_args);

    cargo.stderr(Stdio::inherit());

    cargo.envs(std::env::vars_os());

    let mut child = cargo
        .spawn()
        .map_err(|e| format!("Failed during the cargo execution. {}", e))?;

    let output = child
        .wait()
        .map_err(|e| format!("Reading stdout failed. {}", e))?;

    if !output.success() {
        exit(1);
    }

    let reader = BufReader::new(child.stdout.unwrap());

    Ok(reader.lines().map(|line| line.unwrap()).collect())
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
    // Skip the first argument, which is the binary name (e.g., "cargo").
    let mut args = std::env::args_os().skip(1).collect::<Vec<OsString>>();

    // If the next argument starts with `+` (toolchain), we skip it too. (e.g., `+nightly`)
    if let Some(arg) = args.get(0) {
        if arg.to_string_lossy().starts_with('+') {
            args = args[1..].to_vec();
        }
    }

    // If the next argument ends with `qtest`, we skip it too. That's us!
    if let Some(arg) = args.get(0) {
        if arg.to_string_lossy().ends_with("qtest") {
            args = args[1..].to_vec();
        }
    }

    // Do we want to watch?
    let mut watch = false;
    if let Some(arg) = args.get(0) {
        if arg.to_string_lossy().eq("--watch") {
            args = args[1..].to_vec();
            watch = true
        }
    };

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

    // Build the final cargo test command.
    let mut cargo = format!(
        "{} test {} -- {}",
        cargo_bin(),
        first_args
            .iter()
            .map(|x| x.to_string_lossy())
            .fold(String::new(), |mut acc, f| {
                acc.push(' ');
                acc.push_str(&f);
                acc
            }),
        second_args
            .iter()
            .map(|x| x.to_string_lossy())
            .fold(String::new(), |mut acc, f| {
                acc.push(' ');
                acc.push_str(&f);
                acc
            }),
    );

    // We do this weird shit here since cargo does not support running multiple
    // tests individually.
    for item in final_items {
        cargo.push_str(" --skip ");
        cargo.push_str(&item);
    }

    cargo.push_str(" --exact");

    if watch {
        cargo = format!("{} watch -- {}", cargo_bin(), cargo);
    }

    let mut cmd = {
        #[cfg(target_family = "windows")]
        {
            let mut cmd = Command::new("cmd");
            cmd.arg("/c");
            cmd
        }

        // Assume `sh` is available everywhere else
        // and let it fail if not
        #[cfg(not(target_family = "windows"))]
        {
            let mut cmd = Command::new("sh");
            cmd.arg("-c");
            cmd
        }
    };

    cmd.envs(std::env::vars_os()).arg(cargo);
    cmd.spawn().unwrap().wait().unwrap();
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
