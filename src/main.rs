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
use std::process::exit;
use std::process::Command;

/// Cargo wrapper to retrieve test metadata.
fn get_cargo_test_output() -> Result<Vec<String>, String> {
    let output = Command::new("cargo")
        .args([
            "test", "--", "--list", "--color", "never", "--format", "terse",
        ])
        .output()
        .map_err(|e| format!("Reading test metadata failed. {}", e))?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .map(|data| data.lines().map(String::from).collect())
            .map_err(|e| format!("Reading stdout failed. {}", e))
    } else {
        eprintln!("Cargo failed with: {:?}", output.status);
        exit(1);
    }
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
        "Select set of tests you wish to execute (at least 1 must be chosen):",
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
    println!("Collecting test files from the project..\n");

    let lines = match get_cargo_test_output() {
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

    let mut cmd = Command::new("cargo");

    cmd.args(["test", "--"]);

    // We do this weird shit here since cargo does not support running multiple
    // tests individually.
    for item in final_items {
        cmd.args(["--skip", &item]);
    }

    cmd.arg("--exact");

    let mut output = cmd.spawn().unwrap();
    output.wait().unwrap();
}

mod tests {
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
}
