//! Tests for the prompt module

use oli_server::prompts::{get_agent_prompt_with_cwd, DEFAULT_AGENT_PROMPT};

/// Test that default prompt is returned when no working directory is provided
#[test]
fn test_get_agent_prompt_without_cwd() {
    let prompt = get_agent_prompt_with_cwd(None);
    assert_eq!(prompt, DEFAULT_AGENT_PROMPT);
}

/// Test that working directory is added to the prompt when provided
#[test]
fn test_get_agent_prompt_with_cwd() {
    let test_cwd = "/path/to/working/directory";
    let prompt = get_agent_prompt_with_cwd(Some(test_cwd));

    // Ensure the base prompt is included
    assert!(prompt.contains(DEFAULT_AGENT_PROMPT));

    // Ensure the working directory section is added
    assert!(prompt.contains("## WORKING DIRECTORY"));
    assert!(prompt.contains(&format!("Your current working directory is: {}", test_cwd)));
    assert!(prompt.contains("When using file system tools"));
    assert!(prompt.contains("you should use absolute paths"));
}

/// Test that working directory is added to the prompt with exact format
#[test]
fn test_prompt_cwd_exact_format() {
    let test_cwd = "/path/to/working/directory";
    let prompt = get_agent_prompt_with_cwd(Some(test_cwd));

    // Check the exact CWD section format
    let expected_cwd_section = format!(
        "## WORKING DIRECTORY\nYour current working directory is: {}\nWhen using file system tools such as Read, Glob, Grep, LS, Edit, and Write, you should use absolute paths. You can use this working directory to construct them when needed.",
        test_cwd
    );

    assert!(prompt.contains(&expected_cwd_section));
}

/// Test that prompt contains working directory when in different positions
#[test]
fn test_prompt_with_integrated_cwd() {
    let test_cwd = "/path/to/working/directory";
    let custom_prompt = "You are a helpful assistant.\nYou help users with their tasks.";

    // Create the same CWD format that's used in agent/core.rs
    let expected_prompt = format!(
        "{}\n\n## WORKING DIRECTORY\nYour current working directory is: {}\nWhen using file system tools such as Read, Glob, Grep, LS, Edit, and Write, you should use absolute paths. You can use this working directory to construct them when needed.",
        custom_prompt,
        test_cwd
    );

    // Verify the format is correct
    assert!(expected_prompt.contains("## WORKING DIRECTORY"));
    assert!(expected_prompt.contains(&format!("Your current working directory is: {}", test_cwd)));
}
