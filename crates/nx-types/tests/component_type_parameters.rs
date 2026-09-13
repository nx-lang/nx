//! Type-checking behavior for component type parameters.
//!
//! One case per scenario in the `component-type-parameters` and `component-contract-inheritance`
//! capabilities, plus the `primitive-type-names` scenario about an unspecified parameter.

use nx_types::{analyze_str, check_str};

fn errors(source: &str) -> Vec<String> {
    check_str(source, "test.nx")
        .errors()
        .iter()
        .map(|diagnostic| diagnostic.message().to_string())
        .collect()
}

fn assert_clean(source: &str) {
    let errors = errors(source);
    assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
}

fn assert_reports(source: &str, needle: &str) -> Vec<String> {
    let errors = errors(source);
    assert!(
        errors.iter().any(|message| message.contains(needle)),
        "expected an error containing {needle:?}, got: {errors:?}"
    );
    errors
}

const CONTACT: &str = "type Contact = { name:string }\n";
const SKIA_LAYOUT: &str = "external component <SkiaLayout TItem:type itemsSource:TItem[]? />\n";
const LIST: &str = "external component <List TItem:type items:TItem[]? />\n";

// ---------------------------------------------------------------------------------------------
// A type parameter is a rigid nominal type within its declaration
// ---------------------------------------------------------------------------------------------

#[test]
fn a_concrete_value_is_not_a_type_parameter() {
    assert_reports(
        "component <Bad TItem:type /> = { state { x:TItem = \"text\" } <Label /> }",
        "expects TItem, found string",
    );
}

#[test]
fn two_type_parameters_are_distinct_types() {
    assert_reports(
        "component <Pair TKey:type TValue:type key:TKey value:TValue /> = { state { v:TValue = {key} } <Label /> }",
        "expects TValue, found TKey",
    );
}

#[test]
fn a_type_parameter_satisfies_object_and_accepts_the_empty_list() {
    assert_clean(
        "component <Ok TItem:type items:TItem[] = {} /> = { state { o:object = {items} } <Label /> }\n\
         let v = <Ok items={} />",
    );
}

#[test]
fn a_type_parameter_shadows_a_same_named_type_inside_the_component() {
    assert_clean(
        "type TItem = { id:int }\n\
         component <List TItem:type items:TItem[] /> = { <Label /> }\n\
         let v = <List TItem=string items={\"a\"} />",
    );
    // With the record as the argument instead, a string no longer fits: the parameter, not the
    // record, is what `items:TItem[]` named.
    assert_reports(
        "type TItem = { id:int }\n\
         component <List TItem:type items:TItem[] /> = { <Label /> }\n\
         let v = <List TItem=TItem items={\"a\"} />",
        "expects TItem[], found string",
    );
}

#[test]
fn a_type_parameter_is_not_visible_outside_its_component() {
    // Outside the declaration the name reaches whatever the module declares under it.
    assert_clean(
        "type TItem = { id:int }\n\
         component <List TItem:type /> = { <Label /> }\n\
         let x:TItem = <TItem id=1 />",
    );
}

#[test]
fn a_type_parameter_is_usable_in_the_component_body() {
    assert_clean(
        "component <First TItem:type items:TItem[] /> = { state { first:TItem? = null } <Label text=\"ok\" /> }",
    );
}

// ---------------------------------------------------------------------------------------------
// A use site supplies a type argument as a bare type name
// ---------------------------------------------------------------------------------------------

#[test]
fn a_type_argument_fixes_the_element_type_of_a_list_prop() {
    assert_clean(&format!(
        "{CONTACT}{SKIA_LAYOUT}let contacts:Contact[] = {{}}\n\
         let v = <SkiaLayout TItem=Contact itemsSource={{contacts}} />"
    ));
}

#[test]
fn a_mismatch_is_reported_against_the_substituted_type() {
    assert_reports(
        &format!("{CONTACT}{SKIA_LAYOUT}let v = <SkiaLayout TItem=Contact itemsSource={{ \"a\" \"b\" }} />"),
        "expects Contact[]?, found list string[]",
    );
}

#[test]
fn a_primitive_an_alias_and_a_union_are_all_acceptable_arguments() {
    assert_clean(&format!(
        "type Fit = fill | cover\ntype Label = string\n{LIST}\
         let a = <List TItem=int items={{ 1 2 }} />\n\
         let b = <List TItem=Label items={{ \"x\" }} />\n\
         let c = <List TItem=Fit items=fill />\n\
         let d = <List TItem=object items={{ 1 \"two\" }} />"
    ));
}

#[test]
fn a_type_parameter_in_an_emitted_action_payload_is_rejected() {
    let errors = assert_reports(
        "component <List TItem:type items:TItem[]? emits { pick { item:TItem } } /> = { <Label /> }",
        "Emitted action 'pick' on component 'List' cannot type its payload field 'item' by type parameter 'TItem'",
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    // An inherited parameter is a type in the derived signature on the same terms.
    assert_reports(
        "abstract component <Base TItem:type items:TItem[]? />\n\
         component <List extends Base emits { pick { item:TItem } } /> = { <Label /> }",
        "Emitted action 'pick' on component 'List' cannot type its payload field 'item' by type parameter 'TItem'",
    );
    // A payload field typed by something else is unaffected.
    assert_clean(
        "component <List TItem:type items:TItem[]? emits { pick { index:int } } /> = { <Label /> }",
    );
}

#[test]
fn an_enclosing_component_forwards_its_own_type_parameter() {
    assert_clean(&format!(
        "{SKIA_LAYOUT}component <Section TItem:type items:TItem[] /> = {{ <SkiaLayout TItem=TItem itemsSource={{items}} /> }}"
    ));
    // The forwarded parameter is the enclosing one: a value of another type does not fit.
    assert_reports(
        &format!(
            "{SKIA_LAYOUT}component <Section TItem:type items:TItem[] labels:string[] /> = {{ <SkiaLayout TItem=TItem itemsSource={{labels}} /> }}"
        ),
        "expects TItem[]?, found list string[]",
    );
}

#[test]
fn a_braced_or_quoted_argument_is_rejected() {
    assert_reports(
        &format!("{CONTACT}{LIST}let a = <List TItem={{Contact}} />"),
        "write TItem=Contact",
    );
    assert_reports(
        &format!("{CONTACT}{LIST}let b = <List TItem=\"Contact\" />"),
        "write TItem=Contact",
    );
}

#[test]
fn a_conditional_argument_is_rejected() {
    assert_reports(
        &format!("{CONTACT}{LIST}let flag = true\nlet v = <List if flag {{ TItem=Contact }} />"),
        "Type parameter 'TItem' on 'List' cannot be bound conditionally",
    );
}

#[test]
fn an_unknown_type_name_suggests_a_near_match() {
    let errors = assert_reports(
        &format!("{CONTACT}{LIST}let v = <List TItem=Contatc />"),
        "expects a type name",
    );
    assert!(
        errors
            .iter()
            .any(|message| message.contains("did you mean `Contact`")),
        "expected a suggestion of `Contact`, got: {errors:?}"
    );
}

#[test]
fn a_value_binding_of_the_same_name_is_not_an_argument() {
    assert_reports(
        &format!("{LIST}let Contact = \"a\"\nlet v = <List TItem=Contact />"),
        "'Contact' is not a visible type",
    );
}

#[test]
fn binding_a_type_argument_on_a_component_without_that_parameter_is_an_unknown_property() {
    assert_reports(
        "component <Button text:string /> = { <Label /> }\nlet v = <Button text=\"ok\" TItem=string />",
        "Element 'Button' has no property 'TItem'",
    );
}

// ---------------------------------------------------------------------------------------------
// An unspecified type parameter is the bottom type
// ---------------------------------------------------------------------------------------------

#[test]
fn a_use_site_that_never_touches_the_parameter_needs_no_argument() {
    assert_clean(
        "external component <SkiaLayout TItem:type itemsSource:TItem[]? content children:object[]? />\n\
         let v = <SkiaLayout><Label /></SkiaLayout>",
    );
}

#[test]
fn an_empty_list_is_accepted_without_an_argument() {
    assert_clean(&format!(
        "{SKIA_LAYOUT}let v = <SkiaLayout itemsSource={{}} />"
    ));
}

#[test]
fn a_non_empty_binding_without_an_argument_names_the_parameter() {
    let errors = assert_reports(
        &format!("{CONTACT}{SKIA_LAYOUT}let contacts:Contact[] = {{}}\nlet v = <SkiaLayout itemsSource={{contacts}} />"),
        "Property 'itemsSource' on 'SkiaLayout' is typed by 'TItem', which was not specified",
    );
    assert!(
        errors.iter().any(|message| message.contains("TItem=")),
        "expected the `TItem=` form, got: {errors:?}"
    );
    assert!(
        !errors.iter().any(|message| message.contains("never")),
        "the bottom type must not be spelled, got: {errors:?}"
    );
}

#[test]
fn each_parameter_falls_back_independently() {
    assert_clean(
        "external component <Grid TRow:type TCol:type rows:TRow[]? cols:TCol[]? />\n\
         let v = <Grid TRow=int rows={ 1 2 } />",
    );
}

#[test]
fn an_unspecified_parameter_failure_and_an_ordinary_mismatch_are_reported_separately() {
    let errors = errors(&format!(
        "{CONTACT}external component <List TItem:type items:TItem[]? count:int />\n\
         let contacts:Contact[] = {{}}\nlet v = <List items={{contacts}} count=\"x\" />"
    ));
    assert!(
        errors
            .iter()
            .any(|message| message.contains("typed by 'TItem', which was not specified")),
        "expected the named diagnostic, got: {errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|message| message.contains("Property 'count' on 'List' expects int, found string")),
        "expected the ordinary mismatch, got: {errors:?}"
    );
    assert_eq!(errors.len(), 2, "got: {errors:?}");
}

// ---------------------------------------------------------------------------------------------
// Type parameters are not props and leave no trace below type checking
// ---------------------------------------------------------------------------------------------

#[test]
fn a_type_parameter_is_not_a_value_in_the_body() {
    let errors = errors("component <List TItem:type /> = { <Label text={TItem} /> }");
    assert!(
        errors.iter().any(|message| message.contains("TItem")),
        "expected `TItem` to be reported as undefined, got: {errors:?}"
    );
}

#[test]
fn a_type_parameter_is_never_a_missing_property() {
    assert_clean("external component <List TItem:type items:TItem[] />\nlet v = <List items={} />");
}

#[test]
fn the_property_union_of_a_generic_stateful_component_is_derived_from_state_only() {
    assert_clean(
        "component <List TItem:type items:TItem[] /> = { state { selected:int = 0 } <Label /> }\n\
         let k:List.Property = {List.Property.selected}",
    );
    assert_reports(
        "component <List TItem:type items:TItem[] /> = { state { selected:int = 0 } <Label /> }\n\
         let k:List.Property = {List.Property.TItem}",
        "TItem",
    );
}

#[test]
fn elements_with_different_arguments_have_the_same_type() {
    assert_clean(&format!(
        "{CONTACT}{LIST}let a:List = <List TItem=Contact />\nlet b:List = <List TItem=int />\nlet c:List = <List />"
    ));
}

#[test]
fn type_arguments_are_removed_from_the_element_and_kept_in_the_analysis_result() {
    let artifact = analyze_str(
        &format!("{CONTACT}{SKIA_LAYOUT}let v = <SkiaLayout TItem=Contact itemsSource={{}} />"),
        "test.nx",
    );
    assert!(artifact.is_ok(), "got: {:?}", artifact.errors());
    let module = artifact
        .lowered_module
        .as_ref()
        .expect("the module should have been lowered");

    let (element_id, element) = module
        .exprs()
        .find_map(|(_, expr)| match expr {
            nx_hir::ast::Expr::Element { element, .. } => {
                let candidate = module.element(*element);
                (candidate.tag.as_str() == "SkiaLayout").then_some((*element, candidate))
            }
            _ => None,
        })
        .expect("the SkiaLayout element");

    let keys = element
        .property_entries()
        .iter()
        .filter_map(|entry| match entry {
            nx_hir::PropertyEntry::Value(property) => Some(property.key.as_str().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(keys, vec!["itemsSource"]);
    assert!(
        element
            .properties
            .iter()
            .all(|property| property.key.as_str() != "TItem"),
        "the flat property list must not carry the type argument either"
    );

    let arguments = artifact
        .element_type_arguments
        .get(&element_id)
        .expect("the element's resolved type arguments");
    let rendered = arguments
        .iter()
        .map(|(name, ty)| format!("{}={}", name, ty))
        .collect::<Vec<_>>();
    assert_eq!(rendered, vec!["TItem=Contact"]);
}

// ---------------------------------------------------------------------------------------------
// Derived components inherit type parameters open
// ---------------------------------------------------------------------------------------------

#[test]
fn a_derived_component_accepts_an_argument_for_an_inherited_type_parameter() {
    assert_clean(&format!(
        "abstract external component <ItemsBase TItem:type itemsSource:TItem[]? />\n\
         external component <ContactList extends ItemsBase spacing:int? />\n\
         {CONTACT}let contacts:Contact[] = {{}}\n\
         let v = <ContactList TItem=Contact itemsSource={{contacts}} spacing=4 />"
    ));
    assert_reports(
        &format!(
            "abstract external component <ItemsBase TItem:type itemsSource:TItem[]? />\n\
             external component <ContactList extends ItemsBase spacing:int? />\n\
             {CONTACT}let v = <ContactList TItem=Contact itemsSource={{ \"a\" }} spacing=4 />"
        ),
        "expects Contact[]?, found string",
    );
}

#[test]
fn a_derived_component_body_sees_the_inherited_type_parameter() {
    assert_clean(
        "abstract component <ItemsBase TItem:type items:TItem[] />\n\
         component <Count extends ItemsBase /> = { state { first:TItem? = null } <Label /> }",
    );
    assert_reports(
        "abstract component <ItemsBase TItem:type items:TItem[] />\n\
         component <Count extends ItemsBase /> = { state { first:TItem = \"x\" } <Label /> }",
        "expects TItem, found string",
    );
}

#[test]
fn a_derived_component_adds_its_own_type_parameter_after_the_inherited_one() {
    assert_clean(
        "abstract component <ItemsBase TItem:type items:TItem[] />\n\
         component <Keyed extends ItemsBase TKey:type keys:TKey[] /> = { <Label /> }\n\
         let v = <Keyed TItem=string TKey=int items={ \"a\" } keys={ 1 } />",
    );
}

#[test]
fn redeclaring_an_inherited_type_parameter_is_rejected() {
    assert_reports(
        "abstract component <ItemsBase TItem:type />\n\
         component <Bad extends ItemsBase TItem:type /> = { <Label /> }",
        "Component 'Bad' redeclares inherited type parameter 'TItem' from 'ItemsBase'",
    );
    assert_reports(
        "abstract component <ItemsBase TItem:type />\n\
         component <Worse extends ItemsBase TItem:string /> = { <Label /> }",
        "Component 'Worse' redeclares inherited type parameter 'TItem' from 'ItemsBase'",
    );
}

// ---------------------------------------------------------------------------------------------
// Declaration-form diagnostics reach the checker's result
// ---------------------------------------------------------------------------------------------

#[test]
fn a_duplicate_type_parameter_or_prop_name_is_rejected() {
    assert_reports(
        "component <Bad TItem:type TItem:type /> = { <Label /> }",
        "Type parameter 'TItem' is declared more than once",
    );
    assert_reports(
        "component <Worse TItem:type TItem:string /> = { <Label /> }",
        "Prop 'TItem' has the same name as a type parameter of component 'Worse'",
    );
}

#[test]
fn a_type_parameter_after_a_regular_prop_is_rejected() {
    assert_reports(
        "component <Bad items:object[] TItem:type /> = { <Label /> }",
        "Type parameter 'TItem' must be declared before every prop",
    );
}

#[test]
fn a_type_parameter_named_after_a_primitive_is_rejected() {
    assert_reports(
        "external component <List string:type items:string[]? />\nlet v = <List string=int items={ 1 } />",
        "Type parameter 'string' cannot take the name of a primitive type",
    );
}
