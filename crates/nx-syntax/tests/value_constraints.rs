use nx_syntax::{parse_str, SyntaxKind, SyntaxNode};

fn count(node: &SyntaxNode, kind: SyntaxKind) -> usize {
    usize::from(node.kind() == kind)
        + node
            .children()
            .map(|child| count(&child, kind))
            .sum::<usize>()
}

fn find_all<'t>(node: &SyntaxNode<'t>, kind: SyntaxKind, out: &mut Vec<SyntaxNode<'t>>) {
    if node.kind() == kind {
        out.push(*node);
    }
    for child in node.children() {
        find_all(&child, kind, out);
    }
}

fn parses(source: &str) -> nx_syntax::ParseResult {
    let parsed = parse_str(source, "constraints.nx");
    assert!(parsed.errors.is_empty(), "{source}: {:?}", parsed.errors);
    parsed
}

#[test]
fn a_constrained_field_is_followed_by_its_default_and_the_next_field() {
    let parsed = parses("type Paint = { opacity: float64 where in 0.0..=1.0 = 1.0  layer: int }");
    let tree = parsed.tree.unwrap();
    let mut fields = Vec::new();
    find_all(&tree.root(), SyntaxKind::PROPERTY_DEFINITION, &mut fields);
    assert_eq!(fields.len(), 2);
    assert!(fields[0]
        .child_by_field("type")
        .unwrap()
        .child_by_field("constraint")
        .is_some());
    assert_eq!(fields[0].child_by_field("default").unwrap().text(), "1.0");
    assert!(fields[1]
        .child_by_field("type")
        .unwrap()
        .child_by_field("constraint")
        .is_none());
}

#[test]
fn constraints_parse_wherever_a_type_is_written() {
    for source in [
        "type Ratio = float32 where in 0.0..=1.0",
        "type Pan = float32 where in -1.0..=1.0",
        "type Unit = float64 where >= 0.0 && < 1.0",
        "type Scores = int+ where >= 0",
        "action Moved = { distance: float64 where >= 0.0 }",
        "type Shape = | circle { radius: float64 where > 0.0 } | dot",
        "let fade(amount: float32 where in 0.0..=1.0, steps: int where >= 1): int where >= 0 = { steps }",
        "let half: float64 where <= 1.0 = 0.5",
        "let <Meter level: float64 where in 0.0..=1.0 /> = <p />",
        "component <Counter step: int where >= 1 = 1 /> = {\n  state { count: int where >= 0 = 0 }\n  <p />\n}",
    ] {
        let tree = parses(source).tree.unwrap();
        assert!(count(&tree.root(), SyntaxKind::WHERE_CLAUSE) >= 1, "{source}");
    }
}

#[test]
fn unsupported_constraint_forms_parse_for_the_checker_to_name() {
    for source in [
        "type Edge = int where < 0 || > 9",
        "type Count = int where value >= 1",
        "type NotZero = int where != 0",
    ] {
        let tree = parses(source).tree.unwrap();
        assert_eq!(count(&tree.root(), SyntaxKind::WHERE_CLAUSE), 1, "{source}");
    }
}

#[test]
fn a_field_or_parameter_may_be_named_where() {
    for source in [
        "type Query = { limit: int  where: string  order?: string }",
        "type Only = { where: string }",
        "let <Filter count: int where?: string /> = <p />",
        "let f(where: int) = { where }",
        "component <Search limit: int where: string /> = { <p /> }",
    ] {
        let tree = parses(source).tree.unwrap();
        assert_eq!(count(&tree.root(), SyntaxKind::WHERE_CLAUSE), 0, "{source}");
    }
    let tree = parses("type Slider = { value: float64 where in 0.0..=1.0  where: string }")
        .tree
        .unwrap();
    let mut fields = Vec::new();
    find_all(&tree.root(), SyntaxKind::PROPERTY_DEFINITION, &mut fields);
    let names: Vec<_> = fields
        .iter()
        .map(|field| field.child_by_field("name").unwrap().text())
        .collect();
    assert_eq!(names, ["value", "where"]);
    assert_eq!(count(&tree.root(), SyntaxKind::WHERE_CLAUSE), 1);
}

fn arm_count(source: &str) -> usize {
    let tree = parses(source).tree.unwrap();
    count(&tree.root(), SyntaxKind::VALUE_IF_MATCH_ARM)
        + count(&tree.root(), SyntaxKind::ELEMENTS_IF_MATCH_ARM)
        + count(&tree.root(), SyntaxKind::PROPERTY_LIST_IF_MATCH_ARM)
}

#[test]
fn numeric_patterns_parse_in_match_arms() {
    assert_eq!(
        arm_count("let describe(x: float64) = { if x is { in 0.0..1.0 => \"valid\" > 1.0 => \"too big\" < 0.0 => \"too small\" else => \"one\" } }"),
        3
    );
    assert_eq!(
        arm_count(
            "let flag(n: int) = { if n is { < 0, > 100 => \"out\" > 0 && <= 10 => \"low\" } }"
        ),
        2
    );
}

#[test]
fn an_unbraced_arm_body_ends_where_the_next_pattern_begins() {
    assert_eq!(
        arm_count("let g(n: int) = { if n is { 1 => 5 -1 => 6 > 9 => 7 else => 8 } }"),
        3
    );
    assert_eq!(
        arm_count("let h(n: int) = { if n is { 1 => n - 1 2 => n + 1 >= 3 => n * 2 <= -4 => 0 < 9 => 1 } }"),
        5
    );
    // A body still holds its own operators when no `=>` follows.
    assert_eq!(
        arm_count("let k(n: int) = { if n is { 1 => n < 3 else => false } }"),
        1
    );
    assert_eq!(
        arm_count("let e(n: int) = <div>{ if n is { 1 => <a/> < 0 => <b/> -1 => <c/> >= 5 => <d/> } }</div>"),
        4
    );
    assert_eq!(
        arm_count("let p(n: int) = <div if n is { 1 => x=5 -1 => x=6 < 0 => x=7 >= 9 => x=8 in 2..4 => } />"),
        5
    );
}
