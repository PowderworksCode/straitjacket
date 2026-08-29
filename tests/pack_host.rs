//! The wasm pack host, against a real treebank pack.
//!
//! These used to run only when `TREEBANK_PACK` pointed at a pack, and nothing
//! ever set it -- so all six passed in CI having loaded nothing and parsed
//! nothing. That is how the host sat requiring pack ABI 2 while treebank
//! shipped 3, refusing every pack in existence with no test to say so. A test
//! that silently passes because it found no grammar is worse than no test,
//! and this file was the example.
//!
//! So the pack is fetched rather than waited for. `treebank::fetch` is the
//! acquisition half of the crate that builds packs -- manifest, sha256, a
//! content-addressed cache -- taken without its engine, since the engine is
//! the thing under test here.
//!
//! Deliberately the *current* python pack rather than a pinned one. It costs
//! reproducibility and couples CI to treebank shipping something loadable,
//! which is the coupling that actually exists: if treebank publishes a pack
//! this host cannot drive, straitjacket is broken in the field and this is
//! where that should surface. A digest is the right pin for a build, not for
//! the test that watches for drift.

use beamte::node::{Node, Unit};
use beamte::role::Role;
use beamte::{TestModel, inspect};
use straitjacket::pack::Pack;

fn pack() -> Pack {
    let bytes = treebank::fetch::fetch_bytes("python")
        .expect("fetching the python pack; these tests exist to drive a real one");
    Pack::from_bytes(&bytes, "the treebank python pack").expect("the pack loads")
}

#[test]
fn a_pack_reports_its_language_and_knows_every_term_it_declares() {
    let pack = pack();

    assert_eq!(pack.language(), "python");
    assert_eq!(
        pack.unknown_terms(),
        &[] as &[String],
        "the pack declares terms beamte has no role for"
    );
}

#[test]
fn table_tier_roles_come_off_the_pack() {
    let pack = pack();
    let tree = pack.parse("while x:\n    pass\n").expect("parses");

    let mut found = false;
    beamte::walk(tree.root(), &mut |node| {
        if node.kind() == "while_statement" {
            found = true;
            assert!(node.has_role(Role::Loop), "while_statement is a _loop");
            assert!(
                node.has_role(Role::Statement),
                "and transitively a _statement"
            );
        }
        beamte::Visit::Descend
    });
    assert!(found, "the tree contains a while_statement");
}

#[test]
fn facet_roles_come_off_the_pack_too() {
    let pack = pack();
    let tree = pack.parse("def f():\n    pass\n").expect("parses");

    let function = tree.root().child(0).expect("a first child");
    assert_eq!(function.kind(), "function_definition");
    assert!(function.has_role(Role::Callable), "_callable is a facet");
}

#[test]
fn fields_survive_the_crossing() {
    let pack = pack();
    let tree = pack.parse("def some_name():\n    pass\n").expect("parses");

    let function = tree.root().child(0).expect("a first child");
    let name = function.child_by_field("name").expect("a name field");
    assert_eq!(name.text(), "some_name");
}

#[test]
fn beamte_finds_a_loop_in_a_test_through_the_pack() {
    let pack = pack();
    let source =
        "def test_registers_every_user():\n    for user in users:\n        forum.register(user)\n";
    let tree = pack.parse(source).expect("parses");

    let unit = Unit::new("fixture.py", tree.source(), tree.root());
    let findings = inspect(&unit, &TestModel::python());

    assert_eq!(findings.len(), 1, "one finding, got {findings:?}");
    assert_eq!(findings[0].rule.as_str(), "test-logic");
    assert_eq!(findings[0].span.line, 2);
}

#[test]
fn a_clean_test_yields_nothing() {
    let pack = pack();
    let source = "def test_registers_alice():\n    forum.register(alice)\n    assert forum.has_registered(alice)\n";
    let tree = pack.parse(source).expect("parses");

    let unit = Unit::new("fixture.py", tree.source(), tree.root());
    assert!(inspect(&unit, &TestModel::python()).is_empty());
}
