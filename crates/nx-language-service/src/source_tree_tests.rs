use super::*;
use nx_syntax::parse_str;
use std::path::{Path, PathBuf};
use std::time::Instant;

const URI: &str = "nx://tenant/form.nx";

fn snapshot_for(source: &str) -> WorkspaceSnapshot {
    WorkspaceSnapshot::from_documents(
        Option::<PathBuf>::None,
        vec![DocumentInput::new(URI, source).with_version(1)],
    )
    .expect("snapshot")
}

/// The source tree of a single-document snapshot, checked against the token rule.
fn tree_for(source: &str) -> SourceTree {
    let tree = snapshot_for(source)
        .source_tree(&DocumentUri::from(URI))
        .expect("source tree");
    let violations = violations(source, &tree);
    assert!(
        violations.is_empty(),
        "{source}\n{}\n{}",
        violations.join("\n"),
        outline(&tree)
    );
    tree
}

/// The question-flow conformance program: a flow in `main.nx` over a library of question kinds
/// in three modules, every module a document of one snapshot.
fn question_flow() -> (WorkspaceSnapshot, String) {
    let root = repository().join("specs/ir-conformance/question-flow");
    let documents = [
        "main.nx",
        "library/answers.nx",
        "library/questions.nx",
        "library/layout.nx",
    ]
    .into_iter()
    .map(|path| {
        let source = std::fs::read_to_string(root.join(path)).expect("question-flow source");
        DocumentInput::new(format!("nx://flow/{path}"), source).with_version(1)
    })
    .collect();
    let snapshot =
        WorkspaceSnapshot::from_documents(Option::<PathBuf>::None, documents).expect("snapshot");
    let source = std::fs::read_to_string(root.join("main.nx")).expect("main.nx");
    (snapshot, source)
}

const FLOW: &str = "nx://flow/main.nx";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The nodes of `role`, in order.
fn nodes(tree: &SourceTree, role: SourceRole) -> Vec<&SourceNode> {
    tree.nodes.iter().filter(|node| node.role == role).collect()
}

/// The one node of `role` named `name`.
fn named<'t>(tree: &'t SourceTree, role: SourceRole, name: &str) -> &'t SourceNode {
    let found = tree
        .nodes
        .iter()
        .filter(|node| node.role == role && node.name.as_deref() == Some(name))
        .collect::<Vec<_>>();
    assert_eq!(
        found.len(),
        1,
        "one {role:?} named {name}:\n{}",
        outline(tree)
    );
    found[0]
}

/// The one node of `role` whose value is `value`.
fn valued<'t>(tree: &'t SourceTree, role: SourceRole, value: &str) -> &'t SourceNode {
    let found = tree
        .nodes
        .iter()
        .filter(|node| node.role == role && node.value.as_deref() == Some(value))
        .collect::<Vec<_>>();
    assert_eq!(
        found.len(),
        1,
        "one {role:?} valued {value}:\n{}",
        outline(tree)
    );
    found[0]
}

fn keyed<'t>(tree: &'t SourceTree, key: &str) -> &'t SourceNode {
    tree.nodes
        .iter()
        .find(|node| node.key == key)
        .unwrap_or_else(|| panic!("no node keyed {key}:\n{}", outline(tree)))
}

fn declaration<'t>(tree: &'t SourceTree, node: &SourceNode) -> &'t SourceDeclaration {
    let index = node
        .declaration
        .unwrap_or_else(|| panic!("{node:?} refers to no declaration"));
    &tree.declarations[index as usize]
}

fn parent<'t>(tree: &'t SourceTree, node: &SourceNode) -> &'t SourceNode {
    &tree.nodes[node.parent.expect("a parent") as usize]
}

fn children<'t>(tree: &'t SourceTree, node: &SourceNode) -> Vec<&'t SourceNode> {
    let index = tree
        .nodes
        .iter()
        .position(|candidate| std::ptr::eq(candidate, node))
        .expect("a node of the tree") as u32;
    tree.nodes
        .iter()
        .filter(|child| child.parent == Some(index))
        .collect()
}

/// The text a node covers.
fn text<'s>(source: &'s str, node: &SourceNode) -> &'s str {
    &source[node.range.start_byte as usize..node.range.end_byte as usize]
}

/// The tree as indented lines, for failure messages.
fn outline(tree: &SourceTree) -> String {
    let mut lines = Vec::new();
    for node in &tree.nodes {
        let mut depth = 0;
        let mut parent = node.parent;
        while let Some(index) = parent {
            depth += 1;
            parent = tree.nodes[index as usize].parent;
        }
        lines.push(format!(
            "{}{:?} {} name={:?} value={:?} type={:?} decl={:?} flags={:?} [{}..{}]",
            "  ".repeat(depth),
            node.role,
            node.key,
            node.name,
            node.value,
            node.ty,
            node.declaration
                .map(|index| tree.declarations[index as usize].name.clone()),
            node.flags,
            node.range.start_byte,
            node.range.end_byte
        ));
    }
    lines.join("\n")
}

// ---------------------------------------------------------------------------------------------
// The token rule
// ---------------------------------------------------------------------------------------------

/// The punctuation and keywords a node of `role` may own directly. Anything else it owns has to
/// be what it carries.
fn punctuation(role: SourceRole) -> &'static [&'static str] {
    const EXPRESSION: &[&str] = &["{", "}", "(", ")"];
    match role {
        SourceRole::Import => &["import", "from", "as", "{", "}", ","],
        SourceRole::Declaration => &[
            "let",
            "type",
            "action",
            "component",
            "abstract",
            "external",
            "export",
            "private",
            "extends",
            "emits",
            "state",
            "=",
            "{",
            "}",
            "<",
            ">",
            "/",
            "(",
            ")",
            ",",
            ":",
            "|",
        ],
        SourceRole::Parameter | SourceRole::StateField | SourceRole::Field => {
            &["content", "?", ":", "=", "{", "}", "(", ")"]
        }
        SourceRole::UnionCase => &["|", "{", "}"],
        SourceRole::Emit => &["extends", "{", "}"],
        SourceRole::Element => &["<", ">", "/", ":", "raw", "{", "}", "(", ")"],
        SourceRole::Attribute => &["=", "{", "}", "(", ")"],
        SourceRole::Embed => &["@{", "}", "{", "(", ")"],
        SourceRole::Empty => &["{", "}"],
        SourceRole::Case => &["."],
        SourceRole::Member => &[".", "?."],
        SourceRole::Operator | SourceRole::Literal => EXPRESSION,
        SourceRole::Call | SourceRole::Sequence => &["{", "}", "(", ")", ","],
        SourceRole::Condition => &["if", "else", "=>", "{", "}", "(", ")", ","],
        SourceRole::Match => &["if", "is", "else", "=>", "{", "}", "(", ")", ","],
        SourceRole::MatchArm => &["=>", ",", "{", "}", "(", ")"],
        SourceRole::Loop => &["for", "in", ",", "{", "}", "(", ")"],
        // `as` renames an imported name.
        SourceRole::Reference => &["as"],
        SourceRole::TypeReference
        | SourceRole::Text
        | SourceRole::Binding
        | SourceRole::Comment
        | SourceRole::DocComment
        | SourceRole::Unparsed => &[],
    }
}

/// Whether `node` carries `token` as its name, its value or its text type.
fn carries(node: &SourceNode, token: &str) -> bool {
    let in_name = node.name.as_deref().is_some_and(|name| {
        name == token || token == "." || name.split('.').any(|part| part == token)
    });
    let in_value = node
        .value
        .as_deref()
        .is_some_and(|value| value.contains(token));
    in_name || in_value || node.text_type.as_deref() == Some(token)
}

/// Every way `tree` breaks the rules a source tree keeps: ranges nested and in order, keys unique,
/// and every token of `source` owned by the smallest node holding it and allowed there.
fn violations(source: &str, tree: &SourceTree) -> Vec<String> {
    let mut violations = Vec::new();
    let line_of = |byte: usize| source[..byte].matches('\n').count() + 1;

    let mut top = Vec::new();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); tree.nodes.len()];
    let mut keys = FxHashSet::default();
    for (index, node) in tree.nodes.iter().enumerate() {
        let (start, end) = (node.range.start_byte, node.range.end_byte);
        if start > end {
            violations.push(format!("node {index} ends before it starts"));
        }
        if !keys.insert(node.key.as_str()) {
            violations.push(format!("key `{}` is shared", node.key));
        }
        if index > 0 && tree.nodes[index - 1].range.start_byte > start {
            violations.push(format!("node {index} `{}` is out of order", node.key));
        }
        let siblings = match node.parent {
            Some(parent) if parent as usize >= index => {
                violations.push(format!("node {index} follows its parent"));
                continue;
            }
            Some(parent) => {
                let outer = &tree.nodes[parent as usize].range;
                if start < outer.start_byte || end > outer.end_byte {
                    violations.push(format!("`{}` is outside its parent", node.key));
                }
                &mut children[parent as usize]
            }
            None => &mut top,
        };
        if let Some(&previous) = siblings.last() {
            if tree.nodes[previous].range.end_byte > start {
                violations.push(format!(
                    "`{}` overlaps its sibling `{}`",
                    node.key, tree.nodes[previous].key
                ));
            }
        }
        siblings.push(index);
    }

    let Some(syntax) = parse_str(source, "check.nx").tree else {
        return violations;
    };
    let mut leaves = Vec::new();
    collect_leaves(syntax.root(), &mut leaves);
    for leaf in leaves {
        let (start, end) = (leaf.start_byte(), leaf.end_byte());
        let mut owner = None;
        let mut candidates = &top;
        while let Some(&found) = candidates.iter().find(|&&candidate| {
            let range = &tree.nodes[candidate].range;
            range.start_byte as usize <= start && end <= range.end_byte as usize
        }) {
            owner = Some(found);
            candidates = &children[found];
        }
        let token = leaf.text();
        let Some(owner) = owner else {
            violations.push(format!(
                "line {}: `{token}` belongs to no node",
                line_of(start)
            ));
            continue;
        };
        let node = &tree.nodes[owner];
        // `content` is a keyword the grammar reads as a name, so a keyword may be named.
        let allowed = punctuation(node.role).contains(&token);
        if !allowed && !carries(node, token) {
            violations.push(format!(
                "line {}: `{token}` belongs to {:?} `{}`, which does not say what it is",
                line_of(start),
                node.role,
                node.key
            ));
        }
    }
    violations
}

fn collect_leaves<'t>(node: SyntaxNode<'t>, into: &mut Vec<SyntaxNode<'t>>) {
    if node.raw().child_count() == 0 {
        if node.end_byte() > node.start_byte() {
            into.push(node);
        }
        return;
    }
    for child in node.children_with_tokens() {
        collect_leaves(child, into);
    }
}

// ---------------------------------------------------------------------------------------------
// The answer
// ---------------------------------------------------------------------------------------------

/// Spec: "The tree agrees with hover".
#[test]
fn the_tree_agrees_with_hover() {
    let (snapshot, source) = question_flow();
    let uri = DocumentUri::from(FLOW);
    let tree = snapshot.source_tree(&uri).expect("source tree");
    let element = keyed(&tree, "roleQuestion.value");
    assert_eq!(element.role, SourceRole::Element);
    assert_eq!(element.ty.as_deref(), Some("SingleChoice"));

    let offset = element.range.start_byte as usize + "<Single".len();
    let prefix = &source[..offset];
    let line = prefix.matches('\n').count() as u32;
    let character = prefix[prefix.rfind('\n').map_or(0, |at| at + 1)..]
        .encode_utf16()
        .count() as u32;
    let hover = snapshot
        .hover(&uri, TextPosition::new(line, character))
        .expect("hover")
        .expect("hover at the tag");
    let entry = declaration(&tree, element);
    assert_eq!(entry.name, "SingleChoice");
    assert!(
        hover.contents.contains("SingleChoice"),
        "hover describes {}",
        hover.contents
    );
    assert_eq!(entry.module, "flow/library/questions.nx");
}

/// Spec: "An unknown document".
#[test]
fn an_unknown_document_fails_as_hover_does() {
    let snapshot = snapshot_for("let a = 1");
    let uri = DocumentUri::from("nx://tenant/missing.nx");
    let tree = snapshot.source_tree(&uri).expect_err("unknown document");
    let hover = snapshot
        .hover(&uri, TextPosition::new(0, 0))
        .expect_err("unknown document");
    assert_eq!(tree, hover);
}

#[test]
fn the_answer_round_trips_through_json() {
    let (snapshot, _) = question_flow();
    let tree = snapshot
        .source_tree(&DocumentUri::from(FLOW))
        .expect("source tree");
    let json = serde_json::to_string(&tree).expect("serialize");
    let back: SourceTree = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, tree);
    assert!(json.contains("\"role\":\"matchArm\"") || json.contains("\"role\":\"condition\""));
    assert!(json.contains("\"startByte\""));
}

// ---------------------------------------------------------------------------------------------
// Order, ranges and keys
// ---------------------------------------------------------------------------------------------

/// Spec: "A parent precedes its children".
#[test]
fn a_parent_precedes_its_children() {
    let tree = tree_for(r#"let a = <Card title="Hi" />"#);
    let roles = tree.nodes.iter().map(|node| node.role).collect::<Vec<_>>();
    assert_eq!(
        roles,
        [
            SourceRole::Declaration,
            SourceRole::Element,
            SourceRole::Attribute,
            SourceRole::Literal
        ]
    );
    assert_eq!(tree.nodes[0].parent, None);
    for index in 1..4 {
        assert_eq!(tree.nodes[index].parent, Some(index as u32 - 1));
    }
}

/// Spec: "Ranges count UTF-16 units and bytes".
#[test]
fn ranges_count_utf16_units_and_bytes() {
    let source = "let pair = { \"😀\" 42 }";
    let tree = tree_for(source);
    let literal = valued(&tree, SourceRole::Literal, "42");
    let prefix = &source[..source.find("42").expect("the literal")];
    assert_eq!(literal.range.start_byte as usize, prefix.len());
    assert_eq!(
        literal.range.start.character as usize,
        prefix.encode_utf16().count()
    );
    // The emoji is four bytes and two UTF-16 units.
    assert_eq!(literal.range.start_byte - literal.range.start.character, 2);
}

/// Spec: "An insertion elsewhere does not move a key".
#[test]
fn an_insertion_elsewhere_does_not_move_a_key() {
    let before = tree_for("let a = 1\nlet b = <Card title=\"Hi\" />");
    let after = tree_for("let a = 1\nlet inserted = { 2 3 }\nlet b = <Card title=\"Hi\" />");
    let title = |tree: &SourceTree| named(tree, SourceRole::Attribute, "title").key.clone();
    assert_eq!(title(&before), "b.value.title");
    assert_eq!(title(&before), title(&after));

    // Every node the edit did not touch keeps its key.
    let keys = |tree: &SourceTree| {
        tree.nodes
            .iter()
            .map(|node| (node.key.clone(), node.role))
            .collect::<FxHashSet<_>>()
    };
    let (before, after) = (keys(&before), keys(&after));
    assert!(before.is_subset(&after), "{before:?} ⊄ {after:?}");
}

/// Spec: "Items are keyed by position".
#[test]
fn items_are_keyed_by_position() {
    let (snapshot, _) = question_flow();
    let tree = snapshot
        .source_tree(&DocumentUri::from(FLOW))
        .expect("source tree");
    let choices = keyed(&tree, "roleQuestion.value.choices");
    let items = children(&tree, choices)
        .into_iter()
        .map(|child| child.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        items,
        [
            "roleQuestion.value.choices[0]",
            "roleQuestion.value.choices[1]",
            "roleQuestion.value.choices[2]",
            "roleQuestion.value.choices[3]"
        ]
    );
}

#[test]
fn no_two_nodes_share_a_key() {
    // Two declarations of one name, as a document mid-edit holds, and comments between items.
    let tree = tree_for("let a = 1\nlet a = 2\n// one\n// two\nlet c = { 1 /* x */ 2 }");
    let keys = tree
        .nodes
        .iter()
        .map(|node| node.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        [
            "a",
            "a.value",
            "a#2",
            "a#2.value",
            "c:comment[0]",
            "c:comment[1]",
            "c",
            "c.value",
            "c.value[0]",
            "c.value[1]:comment[0]",
            "c.value[1]"
        ]
    );
}

// ---------------------------------------------------------------------------------------------
// Roles
// ---------------------------------------------------------------------------------------------

#[test]
fn an_import_holds_its_path_and_names() {
    let source = "import \"./ui.nx\" as ui\nimport { Button as B, Card } from \"./kit.nx\"";
    let tree = tree_for(source);
    let imports = nodes(&tree, SourceRole::Import);
    assert_eq!(imports.len(), 2);
    assert_eq!(imports[0].name.as_deref(), Some("ui"));
    assert_eq!(imports[0].key, "import \"./ui.nx\"");
    let path = valued(&tree, SourceRole::Literal, "\"./ui.nx\"");
    assert_eq!(path.key, "import \"./ui.nx\".path");
    let button = named(&tree, SourceRole::Reference, "Button");
    assert_eq!(button.value.as_deref(), Some("B"));
    named(&tree, SourceRole::Reference, "Card");
}

#[test]
fn a_declaration_has_its_name_flags_and_entry() {
    let tree = tree_for(
        "/// A card.\nexport type Card = { title:string }\nprivate abstract external component <Base />",
    );
    let card = named(&tree, SourceRole::Declaration, "Card");
    assert_eq!(card.flags, [SourceFlag::Export]);
    let entry = declaration(&tree, card);
    assert_eq!(entry.kind, SourceDeclarationKind::Record);
    assert_eq!(entry.doc.as_deref(), Some("A card."));
    assert_eq!(entry.range, Some(card.range));
    let base = named(&tree, SourceRole::Declaration, "Base");
    assert_eq!(
        base.flags,
        [
            SourceFlag::Private,
            SourceFlag::Abstract,
            SourceFlag::External
        ]
    );
    assert_eq!(
        declaration(&tree, base).kind,
        SourceDeclarationKind::Component
    );
    let doc = &nodes(&tree, SourceRole::DocComment)[0];
    assert_eq!(doc.value.as_deref(), Some("/// A card."));
    assert_eq!(doc.key, "Card:docComment[0]");
}

#[test]
fn members_have_their_roles() {
    let tree = tree_for(
        "type Mode = light | dark { level:int }\n\
         action Saved = { id:string }\n\
         type Card = { title?:string = \"x\" content body:string }\n\
         let f(x:int) = { x }\n\
         component <Counter start:int = 0 emits { Done { total:int } Saved } /> = {\n\
           state { count:int = {start} }\n\
           <div />\n\
         }",
    );
    let title = named(&tree, SourceRole::Field, "title");
    assert_eq!(title.flags, [SourceFlag::Optional]);
    let default = keyed(&tree, "Card.title.default");
    assert_eq!(default.value.as_deref(), Some("\"x\""));
    assert_eq!(
        named(&tree, SourceRole::Field, "body").flags,
        [SourceFlag::Content]
    );
    named(&tree, SourceRole::Field, "id");
    assert_eq!(keyed(&tree, "Mode.dark.level").role, SourceRole::Field);
    assert_eq!(
        named(&tree, SourceRole::UnionCase, "light").key,
        "Mode.light"
    );
    named(&tree, SourceRole::Parameter, "x");
    named(&tree, SourceRole::Parameter, "start");
    let count = named(&tree, SourceRole::StateField, "count");
    assert_eq!(count.key, "Counter.count");
    assert_eq!(
        keyed(&tree, "Counter.count.default").flags,
        [SourceFlag::Braced]
    );
    named(&tree, SourceRole::Emit, "Done");
    assert_eq!(keyed(&tree, "Counter.Done.total").role, SourceRole::Field);
    let saved = named(&tree, SourceRole::Emit, "Saved");
    assert_eq!(
        declaration(&tree, saved).kind,
        SourceDeclarationKind::Action
    );
}

#[test]
fn a_type_reference_is_spelled_in_nx() {
    let tree =
        tree_for("type Card = { title:string }\nlet f(cards: Card+ , n:int?): string = { \"\" }");
    let cards = keyed(&tree, "f.cards.type");
    assert_eq!(cards.role, SourceRole::TypeReference);
    assert_eq!(cards.value.as_deref(), Some("Card+"));
    assert_eq!(declaration(&tree, cards).name, "Card");
    assert_eq!(keyed(&tree, "f.n.type").value.as_deref(), Some("int?"));
    assert_eq!(keyed(&tree, "f.type").value.as_deref(), Some("string"));
}

/// Spec: "An element and its attributes".
#[test]
fn an_element_and_its_attributes() {
    let (snapshot, source) = question_flow();
    let tree = snapshot
        .source_tree(&DocumentUri::from(FLOW))
        .expect("source tree");
    assert!(violations(&source, &tree).is_empty());
    let element = keyed(&tree, "roleQuestion.value");
    assert_eq!(element.name.as_deref(), Some("SingleChoice"));
    assert_eq!(declaration(&tree, element).name, "SingleChoice");
    let attributes = children(&tree, element)
        .into_iter()
        .filter(|child| child.role == SourceRole::Attribute)
        .map(|child| (child.name.clone().unwrap_or_default(), child.flags.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        attributes,
        [
            ("id".to_string(), vec![]),
            ("label".to_string(), vec![]),
            ("layout".to_string(), vec![]),
            ("allowsOther".to_string(), vec![]),
            ("choices".to_string(), vec![SourceFlag::Content]),
        ]
    );
    let id = keyed(&tree, "roleQuestion.value.id.value");
    assert_eq!(
        (id.role, id.value.as_deref()),
        (SourceRole::Literal, Some("\"role\""))
    );
    let chips = keyed(&tree, "roleQuestion.value.layout.value");
    assert_eq!(chips.role, SourceRole::Case);
    assert_eq!(chips.name.as_deref(), Some("chips"));
    assert_eq!(declaration(&tree, chips).name, "ChoiceLayout");
    let allows = keyed(&tree, "roleQuestion.value.allowsOther.value");
    assert_eq!(allows.role, SourceRole::Literal);
    assert_eq!(allows.value.as_deref(), Some("true"));
    assert_eq!(allows.flags, [SourceFlag::Braced]);
}

#[test]
fn an_element_without_a_content_property_holds_its_items() {
    let tree = tree_for("let a = <div><span /><span /></div>");
    let div = named(&tree, SourceRole::Element, "div");
    let items = children(&tree, div)
        .into_iter()
        .map(|child| child.key.as_str())
        .collect::<Vec<_>>();
    assert_eq!(items, ["a.value[0]", "a.value[1]"]);
}

#[test]
fn text_and_embeds_carry_their_text_type() {
    let source = "let a = <Note:markdown>Hello @{name} &amp; bye</Note>\nlet name = \"x\"\n\
                  let b = <p:>Plain <b>bold</b></p>\nlet c = <Code:raw>if x > 1 {</Code>";
    let tree = tree_for(source);
    let note = named(&tree, SourceRole::Element, "Note");
    assert_eq!(note.text_type.as_deref(), Some("markdown"));
    let hello = valued(&tree, SourceRole::Text, "Hello ");
    assert_eq!(hello.text_type.as_deref(), Some("markdown"));
    let embed = &nodes(&tree, SourceRole::Embed)[0];
    assert_eq!(text(source, embed), "@{name}");
    assert_eq!(children(&tree, embed)[0].name.as_deref(), Some("name"));
    valued(&tree, SourceRole::Text, " &amp; bye");
    valued(&tree, SourceRole::Text, "Plain ");
    named(&tree, SourceRole::Element, "b");
    let code = named(&tree, SourceRole::Element, "Code");
    assert_eq!(code.flags, [SourceFlag::Raw]);
    let raw = valued(&tree, SourceRole::Text, "if x > 1 {");
    assert_eq!(raw.flags, [SourceFlag::Raw]);
}

#[test]
fn literals_and_the_empty_value() {
    let tree = tree_for(
        "type Box = { n:float64 }\nlet a = { 1 2.5 \"s\" true () }\nlet b = <Box n=-2.5 />\nlet e = {}",
    );
    for value in ["1", "2.5", "\"s\"", "true", "()"] {
        let literal = valued(&tree, SourceRole::Literal, value);
        assert_eq!(parent(&tree, literal).role, SourceRole::Sequence);
    }
    assert_eq!(
        keyed(&tree, "b.value.n.value").value.as_deref(),
        Some("-2.5")
    );
    assert_eq!(keyed(&tree, "e.value").role, SourceRole::Empty);
}

#[test]
fn cases_bare_and_qualified_name_their_union() {
    let tree = tree_for(
        "type Mode = light | dark\nlet <Panel mode:Mode /> = <div />\n\
         let a = <Panel mode=dark />\nlet b = { Mode.light }",
    );
    let dark = named(&tree, SourceRole::Case, "dark");
    assert_eq!(dark.ty.as_deref(), Some("Mode.dark"));
    assert_eq!(declaration(&tree, dark).kind, SourceDeclarationKind::Union);
    let light = named(&tree, SourceRole::Case, "light");
    assert_eq!(declaration(&tree, light).name, "Mode");
    let union = &children(&tree, light)[0];
    assert_eq!(union.role, SourceRole::Reference);
    assert_eq!(union.key, "b.value.union");
    assert_eq!(declaration(&tree, union).name, "Mode");
    let entry = declaration(&tree, light);
    let cases = entry
        .cases
        .iter()
        .map(|case| case.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(cases, ["light", "dark"]);
}

#[test]
fn references_and_members() {
    let tree = tree_for(
        "type Card = { title:string }\nlet card = <Card title=\"x\" />\n\
         let f(c:Card) = { c.title }\nlet g = { card?.title }",
    );
    let member = keyed(&tree, "f.body");
    assert_eq!(member.role, SourceRole::Member);
    assert_eq!(member.name.as_deref(), Some("title"));
    assert_eq!(member.ty.as_deref(), Some("string"));
    let parameter = keyed(&tree, "f.body.object");
    assert_eq!(parameter.role, SourceRole::Reference);
    // A parameter is local: it refers to no declaration.
    assert_eq!(parameter.declaration, None);
    assert_eq!(parameter.ty.as_deref(), Some("Card"));
    let optional = keyed(&tree, "g.value");
    assert_eq!(optional.flags, [SourceFlag::Braced, SourceFlag::Optional]);
    let card = keyed(&tree, "g.value.object");
    assert_eq!(declaration(&tree, card).kind, SourceDeclarationKind::Value);
}

/// Spec: "An operator keeps its token".
#[test]
fn an_operator_keeps_its_token() {
    let tree = tree_for(
        "let f(step:int, role:string) = { if step == 3 && role != \"engineer\" { 1 } else { 2 } }",
    );
    let condition = keyed(&tree, "f.body");
    assert_eq!(condition.role, SourceRole::Condition);
    let test = keyed(&tree, "f.body.test");
    assert_eq!(
        (test.role, test.value.as_deref()),
        (SourceRole::Operator, Some("&&"))
    );
    let operands = children(&tree, test)
        .into_iter()
        .map(|child| (child.role, child.value.clone().unwrap_or_default()))
        .collect::<Vec<_>>();
    assert_eq!(
        operands,
        [
            (SourceRole::Operator, "==".to_string()),
            (SourceRole::Operator, "!=".to_string())
        ]
    );
    keyed(&tree, "f.body.then[0]");
    keyed(&tree, "f.body.else[0]");
}

#[test]
fn prefix_postfix_and_ranges_are_operators() {
    let tree = tree_for("let f(a:int?, b:boolean) = { a? && !b }\nlet r = { 1..=3 }");
    valued(&tree, SourceRole::Operator, "?");
    valued(&tree, SourceRole::Operator, "!");
    valued(&tree, SourceRole::Operator, "..=");
}

#[test]
fn a_call_holds_its_callee_and_arguments() {
    let tree = tree_for("let f(x:int, y:int): int = { x + y }\nlet a = { f(1, (2)) }");
    let call = keyed(&tree, "a.value");
    assert_eq!(call.role, SourceRole::Call);
    assert_eq!(call.ty.as_deref(), Some("int"));
    assert_eq!(keyed(&tree, "a.value.callee").name.as_deref(), Some("f"));
    assert_eq!(keyed(&tree, "a.value.args[0]").value.as_deref(), Some("1"));
    assert_eq!(
        keyed(&tree, "a.value.args[1]").flags,
        [SourceFlag::Parenthesized]
    );
}

#[test]
fn matches_and_condition_lists_have_arms() {
    let tree = tree_for(
        "type Mode = light | dark\n\
         let m(k:Mode) = { if k is { Mode.dark => 1 light => 2 } }\n\
         let n(x:int) = { if x is { 1, 2 => \"low\" else => \"high\" } }\n\
         let l(x:int) = { if { x > 1 => 1 else => 2 } }",
    );
    let matching = keyed(&tree, "m.body");
    assert_eq!(matching.role, SourceRole::Match);
    assert_eq!(keyed(&tree, "m.body.subject").name.as_deref(), Some("k"));
    let dark = keyed(&tree, "m.body.is Mode.dark");
    assert_eq!(dark.role, SourceRole::MatchArm);
    assert_eq!(
        keyed(&tree, "m.body.is Mode.dark.pattern[0]").role,
        SourceRole::Case
    );
    assert_eq!(
        keyed(&tree, "m.body.is Mode.dark[0]").value.as_deref(),
        Some("1")
    );
    keyed(&tree, "m.body.is light");
    keyed(&tree, "n.body.is 1, 2.pattern[1]");
    keyed(&tree, "n.body.else[0]");
    let arm = keyed(&tree, "l.body.when x > 1");
    assert_eq!(arm.role, SourceRole::MatchArm);
    assert_eq!(parent(&tree, arm).role, SourceRole::Condition);
    keyed(&tree, "l.body.when x > 1.test");
}

#[test]
fn a_loop_holds_its_bindings_iterable_and_body() {
    let tree = tree_for("let xs = { 1 2 }\nlet a = { for x, i in xs { x } }");
    let looped = keyed(&tree, "a.value");
    assert_eq!(looped.role, SourceRole::Loop);
    assert_eq!(keyed(&tree, "a.value.x").role, SourceRole::Binding);
    assert_eq!(keyed(&tree, "a.value.i").role, SourceRole::Binding);
    assert_eq!(keyed(&tree, "a.value.in").name.as_deref(), Some("xs"));
    assert_eq!(keyed(&tree, "a.value[0]").name.as_deref(), Some("x"));
}

#[test]
fn property_conditions_choose_attributes() {
    let tree = tree_for(
        "type Mode = light | dark\nlet <Panel mode:Mode /> = <div />\n\
         let f(on:boolean) = <Panel if on { mode=light } else { mode=dark } />",
    );
    let condition = keyed(&tree, "f.body.if[0]");
    assert_eq!(condition.role, SourceRole::Condition);
    assert_eq!(
        keyed(&tree, "f.body.if[0].then.mode").role,
        SourceRole::Attribute
    );
    assert_eq!(
        keyed(&tree, "f.body.if[0].else.mode").role,
        SourceRole::Attribute
    );
}

/// Spec: "A handler attribute".
#[test]
fn a_handler_attribute() {
    let tree = tree_for(
        "component <Button emits { Tapped {} } />\n\
         component <Counter emits { Done {} } /> = {\n\
           state { count:int = 0 }\n\
           <Button onTapped=<Update count={count + 1} /> />\n\
         }",
    );
    let handler = named(&tree, SourceRole::Attribute, "onTapped");
    assert_eq!(handler.flags, [SourceFlag::Handler]);
    let update = named(&tree, SourceRole::Element, "Update");
    assert_eq!(update.flags, [SourceFlag::StateUpdate]);
    assert_eq!(
        update.parent,
        keyed(&tree, "Counter.body.onTapped").parent.map(|_| {
            tree.nodes
                .iter()
                .position(|node| node.key == "Counter.body.onTapped")
                .unwrap() as u32
        })
    );
    let count = keyed(&tree, "Counter.body.onTapped.value.count.value.left");
    assert_eq!(count.name.as_deref(), Some("count"));
    assert_eq!(count.declaration, None);
}

#[test]
fn a_handler_of_a_library_component() {
    let (snapshot, _) = question_flow();
    let tree = snapshot
        .source_tree(&DocumentUri::from(FLOW))
        .expect("source tree");
    let step = keyed(&tree, "Flow.body.children[1].then[0]");
    assert_eq!(declaration(&tree, step).module, "flow/library/layout.nx");
    let handler = keyed(&tree, "Flow.body.children[1].then[0].onTextAnswered");
    assert_eq!(handler.flags, [SourceFlag::Handler]);
    let update = keyed(&tree, "Flow.body.children[1].then[0].onTextAnswered.value");
    assert_eq!(update.flags, [SourceFlag::StateUpdate]);
    // `action` names the handled action, a local binding of the handler.
    let action = keyed(
        &tree,
        "Flow.body.children[1].then[0].onTextAnswered.value.name.value.object.object",
    );
    assert_eq!(action.name.as_deref(), Some("action"));
    assert_eq!(action.declaration, None);
}

/// Spec: "Comments are nodes".
#[test]
fn comments_are_nodes() {
    let tree = tree_for("// The answer.\nlet a = 42 /* inline */\n<!-- markup -->");
    let comment = valued(&tree, SourceRole::Comment, "// The answer.");
    assert_eq!(comment.parent, None);
    assert_eq!(comment.key, "a:comment[0]");
    valued(&tree, SourceRole::Comment, "/* inline */");
    valued(&tree, SourceRole::Comment, "<!-- markup -->");
}

/// Spec: "A document that does not parse".
#[test]
fn a_document_that_does_not_parse() {
    let source = "let a = 1\n%%% not nx\nlet b = 2";
    let tree = tree_for(source);
    named(&tree, SourceRole::Declaration, "a");
    let unparsed = &nodes(&tree, SourceRole::Unparsed)[0];
    assert!(text(source, unparsed).contains("%%% not nx"));
    assert_eq!(unparsed.value.as_deref(), Some(text(source, unparsed)));
}

// ---------------------------------------------------------------------------------------------
// The declaration table
// ---------------------------------------------------------------------------------------------

/// Spec: "A library type is described".
#[test]
fn a_library_type_is_described() {
    let (snapshot, _) = question_flow();
    let tree = snapshot
        .source_tree(&DocumentUri::from(FLOW))
        .expect("source tree");
    let entry = declaration(&tree, keyed(&tree, "roleQuestion.value"));
    assert_eq!(entry.module, "flow/library/questions.nx");
    assert_eq!(entry.range, None);
    let properties = entry
        .properties
        .iter()
        .map(|property| property.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        properties,
        [
            "id",
            "label",
            "help",
            "required",
            "requiredMessage",
            "allowsOther",
            "layout",
            "choices"
        ]
    );
    let property = |name: &str| {
        entry
            .properties
            .iter()
            .find(|property| property.name == name)
            .expect("property")
    };
    assert_eq!(property("required").default.as_deref(), Some("true"));
    assert!(property("required").flags.contains(&SourceFlag::Inherited));
    assert_eq!(
        property("help").flags,
        [SourceFlag::Optional, SourceFlag::Inherited]
    );
    assert_eq!(property("choices").flags, [SourceFlag::Content]);
    assert_eq!(property("choices").ty, "Choice+");
    assert_eq!(
        property("layout").default.as_deref(),
        Some("ChoiceLayout.list")
    );
    let bases = entry
        .bases
        .iter()
        .map(|base| tree.declarations[*base as usize].name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(bases, ["Question", "Node"]);
}

/// Spec: "A reference leads to its value".
#[test]
fn a_reference_leads_to_its_value() {
    let (snapshot, _) = question_flow();
    let tree = snapshot
        .source_tree(&DocumentUri::from(FLOW))
        .expect("source tree");
    let reference = tree
        .nodes
        .iter()
        .find(|node| {
            node.role == SourceRole::Reference && node.name.as_deref() == Some("teamSizeQuestion")
        })
        .expect("a reference to teamSizeQuestion");
    let entry = declaration(&tree, reference);
    assert_eq!(entry.name, "teamSizeQuestion");
    assert_eq!(entry.kind, SourceDeclarationKind::Value);
    let element = keyed(&tree, "teamSizeQuestion.value");
    assert_eq!(element.name.as_deref(), Some("Integer"));
    assert_eq!(entry.value_range.as_ref(), Some(&element.range));
    assert_eq!(entry.ty.as_deref(), Some("Integer"));
}

#[test]
fn a_component_lists_its_state_and_an_alias_its_type() {
    let tree = tree_for(
        "type Count = int\ncomponent <Counter\n  /// Where it starts.\n  start:Count = 0\n/> = {\n\
           state { count:int = {start} }\n  <div />\n}\nlet c = <Counter start=1 />",
    );
    let counter = declaration(&tree, named(&tree, SourceRole::Element, "Counter"));
    assert_eq!(counter.properties[0].name, "start");
    assert_eq!(counter.properties[0].ty, "Count");
    assert_eq!(counter.properties[0].default.as_deref(), Some("0"));
    assert_eq!(
        counter.properties[0].doc.as_deref(),
        Some("Where it starts.")
    );
    assert_eq!(counter.state[0].name, "count");
    // A default is the value as written, inside any braces around it.
    assert_eq!(counter.state[0].default.as_deref(), Some("start"));
    let alias = declaration(&tree, keyed(&tree, "Counter.start.type"));
    assert_eq!(alias.kind, SourceDeclarationKind::Alias);
    assert_eq!(alias.ty.as_deref(), Some("int"));
}

#[test]
fn a_standard_library_declaration_is_described() {
    let tree = tree_for("let a = { [1, 2].count }");
    // Whatever the standard library answers, a table entry from another module has no range.
    for entry in &tree.declarations {
        if entry.module != "tenant/form.nx" {
            assert_eq!(entry.range, None);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Coverage over the repository
// ---------------------------------------------------------------------------------------------

/// Spec: "Coverage over the repository" and "A forgotten construct fails the test".
#[test]
fn every_token_of_every_nx_file_belongs_to_a_node() {
    let root = repository().canonicalize().expect("repository root");
    let mut files = Vec::new();
    collect_nx_files(&root, &mut files);
    files.sort();
    assert!(files.len() > 50, "found only {} .nx files", files.len());

    let mut failures = Vec::new();
    for file in &files {
        let relative = file.strip_prefix(&root).expect("under the root");
        let source = std::fs::read_to_string(file).expect("readable .nx file");
        let uri = format!("nx://repo/{}", relative.display());
        let snapshot = WorkspaceSnapshot::from_documents(
            Option::<PathBuf>::None,
            vec![DocumentInput::new(uri.as_str(), source.as_str())],
        )
        .expect("snapshot");
        let tree = snapshot
            .source_tree(&DocumentUri::from(uri.as_str()))
            .expect("source tree");
        for violation in violations(&source, &tree) {
            failures.push(format!("{}: {violation}", relative.display()));
        }
    }
    assert!(
        failures.is_empty(),
        "{} violations in {} files:\n{}",
        failures.len(),
        files.len(),
        failures.join("\n")
    );
}

fn collect_nx_files(directory: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            let skipped = name.starts_with('.')
                || matches!(
                    name.as_str(),
                    "node_modules" | "target" | "dist" | "bin" | "obj"
                );
            if !skipped {
                collect_nx_files(&path, into);
            }
        } else if name.ends_with(".nx") {
            into.push(path);
        }
    }
}

/// Task 2.2: the size and time of the answer for a program of the size a host runs. Run in
/// release to compare with the budget:
/// `cargo test -p nx-language-service --release --lib measure_question_flow -- --nocapture`.
#[test]
fn measure_question_flow_source_tree() {
    let (snapshot, _) = question_flow();
    let uri = DocumentUri::from(FLOW);
    let started = Instant::now();
    let tree = snapshot.source_tree(&uri).expect("source tree");
    let cold = started.elapsed();
    let started = Instant::now();
    let again = snapshot.source_tree(&uri).expect("source tree");
    let warm = started.elapsed();
    assert_eq!(tree, again);
    let json = serde_json::to_string(&tree).expect("serialize");
    println!(
        "question-flow main.nx source tree: {} nodes, {} declarations, {} bytes of JSON; \
         {:.1} ms with the workspace analysis, {:.1} ms after it",
        tree.nodes.len(),
        tree.declarations.len(),
        json.len(),
        cold.as_secs_f64() * 1000.0,
        warm.as_secs_f64() * 1000.0
    );
}
