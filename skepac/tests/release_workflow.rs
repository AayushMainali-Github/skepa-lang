use std::fs;

#[test]
fn release_workflow_dispatch_requires_and_uses_a_tag() {
    let workflow_path = format!(
        "{}/../.github/workflows/release.yml",
        env!("CARGO_MANIFEST_DIR")
    );
    let workflow = fs::read_to_string(workflow_path)
        .expect("release workflow should exist")
        .replace("\r\n", "\n");

    assert!(
        workflow.contains("workflow_dispatch:\n    inputs:\n      tag_name:"),
        "manual releases must require an explicit tag input"
    );
    assert!(
        workflow.contains("required: true\n        type: string"),
        "the manual release tag input must be required"
    );
    assert!(
        workflow.contains(
            "if: startsWith(github.ref, 'refs/tags/') || (github.event_name == 'workflow_dispatch' && inputs.tag_name != '')"
        ),
        "publishing must be gated to tag pushes or explicit dispatches"
    );
    assert!(
        workflow.contains("tag_name: ${{")
            && workflow.contains("inputs.tag_name")
            && workflow.contains("github.ref_name"),
        "the release action must receive the dispatch tag or pushed tag"
    );
}
