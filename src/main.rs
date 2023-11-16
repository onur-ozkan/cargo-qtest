use inquire::formatter::MultiOptionFormatter;
use inquire::list_option::ListOption;
use inquire::validator::Validation;
use inquire::MultiSelect;
use regex::Regex;
use std::process::exit;
use std::process::Command;

fn get_cargo_test_output() -> Result<String, String> {
    let output = Command::new("cargo")
        .args([
            "test", "--", "--list", "--color", "never", "--format", "terse",
        ])
        .output()
        .map_err(|e| format!("Reading test metadata failed. {}", e))?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|e| format!("Reading stdout failed. {}", e))
    } else {
        eprintln!("Cargo failed with: {:?}", output.status);
        exit(1);
    }
}

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

fn main() {
    let lines = match get_cargo_test_output() {
        Ok(stdout) => stdout.lines().map(String::from).collect(),
        Err(err) => {
            eprintln!("{}", err);
            exit(1);
        }
    };

    let options = filter_test_options(lines);

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

    let answer = MultiSelect::new(
        "Select set of tests you wish to execute (at least 1 must be chosen):",
        options.clone(),
    )
    .with_validator(validator)
    .with_formatter(formatter)
    .with_page_size(20)
    .prompt()
    .unwrap();

    let final_items: Vec<String> = options
        .iter()
        .filter(|&value| !answer.contains(value))
        .cloned()
        .collect();

    let mut cmd = Command::new("cargo");

    cmd.args(["test", "--"]);

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
        let input = vec![
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

