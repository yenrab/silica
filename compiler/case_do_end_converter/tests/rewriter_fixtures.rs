use case_do_end_converter::rewrite_case_do_end;

fn assert_fixture(name: &str, input: &str, expected: &str) {
    let rewritten = rewrite_case_do_end(input, &format!("{name}/input.silica"))
        .expect("fixture should rewrite successfully");

    assert_eq!(rewritten, expected);
}

#[test]
fn rewrites_nested_case_branch_do_end_blocks() {
    assert_fixture(
        "nested_case_branch_do_end",
        include_str!("fixtures/nested_case_branch_do_end/input.silica"),
        include_str!("fixtures/nested_case_branch_do_end/expected.silica"),
    );
}

#[test]
fn removes_standalone_do_end_blocks() {
    assert_fixture(
        "standalone_do_end_removed",
        include_str!("fixtures/standalone_do_end_removed/input.silica"),
        include_str!("fixtures/standalone_do_end_removed/expected.silica"),
    );
}

#[test]
fn leaves_already_braced_case_branches_unchanged() {
    assert_fixture(
        "already_braced_case_branch",
        include_str!("fixtures/already_braced_case_branch/input.silica"),
        include_str!("fixtures/already_braced_case_branch/expected.silica"),
    );
}

#[test]
fn leaves_plain_expression_case_branches_unchanged() {
    assert_fixture(
        "plain_expression_case_branch",
        include_str!("fixtures/plain_expression_case_branch/input.silica"),
        include_str!("fixtures/plain_expression_case_branch/expected.silica"),
    );
}

#[test]
fn removes_nested_standalone_do_end_blocks() {
    assert_fixture(
        "nested_standalone_do_end_removed",
        include_str!("fixtures/nested_standalone_do_end_removed/input.silica"),
        include_str!("fixtures/nested_standalone_do_end_removed/expected.silica"),
    );
}

#[test]
fn leaves_do_end_inside_strings_and_comments_unchanged() {
    assert_fixture(
        "string_and_comment_do_end_unchanged",
        include_str!("fixtures/string_and_comment_do_end_unchanged/input.silica"),
        include_str!("fixtures/string_and_comment_do_end_unchanged/expected.silica"),
    );
}

#[test]
fn rewrites_case_branch_do_and_removes_inner_standalone_do() {
    assert_fixture(
        "case_branch_do_with_inner_standalone_do",
        include_str!("fixtures/case_branch_do_with_inner_standalone_do/input.silica"),
        include_str!("fixtures/case_branch_do_with_inner_standalone_do/expected.silica"),
    );
}
