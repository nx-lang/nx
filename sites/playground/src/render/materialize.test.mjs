/**
 * Building DrawnUI controls from evaluated values outside React, and the template cell that does
 * it per bound item — against the real wasm compiler and the site's own catalog.
 *
 * DrawnUI controls are built but never drawn here: construction, prop assignment and children need
 * no canvas, which is what these cases look at.
 */
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";
import { after, test } from "node:test";
import { createNxHost, loadNxModule } from "@nx-lang/sdk-wasm";
import { compileWithCatalog, emitCatalogArtifact } from "../compile/catalog.ts";
import { evaluateRoot, prepare, prepareCatalog } from "./evaluate.ts";
import { materialize } from "./materialize.ts";
import { NxTemplateCell, templateFactory } from "./templateCell.ts";
import { coerceProps, components } from "./values.ts";

const catalog = readFileSync(new URL("../../catalog/skia.nx", import.meta.url), "utf8");
const host = createNxHost(await loadNxModule());
const prepared = prepareCatalog(emitCatalogArtifact(host, catalog));
after(() => host.dispose());

/** Compiles a snippet against the catalog and evaluates its root. */
function open(source) {
  const result = compileWithCatalog(host, catalog, source);
  assert.deepEqual(result.diagnostics.map((d) => d.message), [], "the snippet compiles");
  const program = prepare(result.ir, prepared);
  return { program, root: evaluateRoot(program) };
}

function reporting() {
  const reports = { unknown: [], inert: [], failures: [] };
  const context = {
    reportUnknown: (type) => reports.unknown.push(type),
    reportInert: (where) => reports.inert.push(where),
    reportTemplateFailure: (where) => reports.failures.push(where),
  };
  return { reports, context };
}

test("materialize builds a nested layout with its props and children in order", () => {
  const { program, root } = open(`
    <SkiaStack Spacing=6 BackgroundColor="#111827">
      <SkiaLabel Text="one" FontSize=15 />
      <SkiaShape Type=Circle WidthRequest=42><SkiaLabel Text="two" /></SkiaShape>
    </SkiaStack>
  `);
  const { reports, context } = reporting();
  const controls = materialize(root, { program, ...context });
  assert.equal(controls.length, 1);
  const [stack] = controls;
  assert.equal(stack.constructor.name, "SkiaStack");
  assert.equal(stack.Spacing, 6);
  assert.equal(stack.BackgroundColor, "#111827");
  assert.deepEqual(
    stack.Views.map((view) => view.constructor.name),
    ["SkiaLabel", "SkiaShape"],
  );
  assert.equal(stack.Views[0].Text, "one");
  assert.equal(stack.Views[0].FontSize, 15);
  assert.equal(stack.Views[1].Type, "Circle");
  assert.equal(stack.Views[1].Views[0].Text, "two");
  assert.deepEqual(reports, { unknown: [], inert: [], failures: [] });
});

test("materialize initializes an authored component and builds what it rendered", () => {
  const { program, root } = open(`
    component <Card extends DrawnNode Title:string /> = {
      <SkiaStack><SkiaLabel Text={Title} /></SkiaStack>
    }
    <SkiaLayer><Card Title="hello" /></SkiaLayer>
  `);
  const { reports, context } = reporting();
  const [layer] = materialize(root, { program, ...context });
  assert.equal(layer.constructor.name, "SkiaLayer");
  assert.equal(layer.Views.length, 1);
  assert.equal(layer.Views[0].constructor.name, "SkiaStack");
  assert.equal(layer.Views[0].Views[0].Text, "hello");
  assert.deepEqual(reports, { unknown: [], inert: [], failures: [] });
});

test("materialize leaves a handler unbound and reports it inert, and reports an unknown control", () => {
  const { program, root } = open(`
    action Tapped = { }
    external component <Widget extends DrawnNode />
    <SkiaStack><SkiaButton Text="go" onTapped=<Tapped /> /><Widget /></SkiaStack>
  `);
  const { reports, context } = reporting();
  const [stack] = materialize(root, { program, ...context });
  assert.equal(stack.Views.length, 1, "the unknown control builds nothing");
  assert.equal(stack.Views[0].Tapped, undefined);
  assert.deepEqual(reports.inert, ["SkiaButton.onTapped"]);
  assert.deepEqual(reports.unknown, ["Widget"]);
});

const TEMPLATED_LIST = `
  type Contact = { Id:int Title:string }
  let <ContactCell Item:Contact Index:int />: DrawnNode =
    <SkiaStack BackgroundColor={if Index % 2 == 1 { "#0B1220" } else { "#111827" }}>
      <SkiaLabel Text={Item.Title} />
      <SkiaLabel Text={"#" + Index} />
    </SkiaStack>
  let contacts = { <Contact Id=1 Title="Ada" /> <Contact Id=2 Title="Kai" /> }
  <SkiaStack TItem=Contact ItemsSource={contacts} ItemTemplate={ContactCell} RecyclingTemplate=Enabled />
`;

test("coerceProps turns a Function record on ItemTemplate into a cell factory and passes ItemsSource through", () => {
  const { program, root } = open(TEMPLATED_LIST);
  assert.deepEqual(components.SkiaStack.templates, { ItemTemplate: ["Item", "Index"] });
  assert.equal(components.SkiaStack.itemsSource, "ItemsSource");
  assert.deepEqual(root.ItemTemplate, { $type: "Function", module: "playground.nx", name: "ContactCell" });
  const { context } = reporting();
  let bound;
  const props = coerceProps(root, undefined, (property, record, params) => {
    bound = { property, record, params };
    return templateFactory(program, record, params, { program, ...context });
  });
  assert.deepEqual(bound, { property: "ItemTemplate", record: root.ItemTemplate, params: ["Item", "Index"] });
  assert.equal(typeof props.ItemTemplate, "function");
  assert.deepEqual(props.ItemsSource, root.ItemsSource, "the items reach DrawnUI as the evaluated values");
  assert.equal(props.ItemsSource[0].$type, "Contact");
  assert.equal(props.RecyclingTemplate, "Enabled");
  assert.ok(props.ItemTemplate() instanceof NxTemplateCell);
  // Without a binder the record is dropped rather than handed to DrawnUI as a plain object.
  assert.equal(coerceProps(root).ItemTemplate, undefined);
});

test("a template cell draws the function's result for the item it is bound to, and again when rebound", () => {
  const { program, root } = open(TEMPLATED_LIST);
  const { reports, context } = reporting();
  const cell = templateFactory(program, root.ItemTemplate, ["Item", "Index"], { program, ...context })();
  assert.equal(cell.Views.length, 0, "an unbound cell is empty");

  cell.ContextIndex = 0;
  cell.BindingContext = root.ItemsSource[0];
  assert.equal(cell.Views.length, 1);
  assert.equal(cell.Views[0].BackgroundColor, "#111827");
  assert.deepEqual(cell.Views[0].Views.map((view) => view.Text), ["Ada", "#0"]);

  // DrawnUI rebinds a recycled cell: the index first, then the item, which redraws it.
  cell.ContextIndex = 1;
  cell.BindingContext = root.ItemsSource[1];
  assert.equal(cell.Views.length, 1, "the previous content is replaced, not appended to");
  assert.equal(cell.Views[0].BackgroundColor, "#0B1220");
  assert.deepEqual(cell.Views[0].Views.map((view) => view.Text), ["Kai", "#1"]);
  assert.deepEqual(reports.failures, []);
});

test("a failing template call reports the index and leaves the cell empty", () => {
  const { program, root } = open(TEMPLATED_LIST);
  const { reports, context } = reporting();
  const cell = templateFactory(program, root.ItemTemplate, ["Item", "Index"], { program, ...context })();
  cell.ContextIndex = 3;
  // An item the function cannot read: `Item.Title` needs a Contact.
  cell.BindingContext = "not a contact";
  assert.equal(cell.Views.length, 0);
  assert.equal(reports.failures.length, 1);
  assert.match(reports.failures[0], /^ContactCell at index 3: /);
});

test("a templated control inside a cell keeps its own template", () => {
  const { program, root } = open(`
    type Contact = { Id:int Title:string }
    let <TagCell Item:Contact Index:int />: SkiaControl = <SkiaLabel Text={Item.Title} />
    let <ContactCell Item:Contact Index:int />: SkiaControl =
      <SkiaStack TItem=Contact ItemsSource={contacts} ItemTemplate={TagCell} />
    let contacts = { <Contact Id=1 Title="Ada" /> <Contact Id=2 Title="Kai" /> }
    <SkiaStack TItem=Contact ItemsSource={contacts} ItemTemplate={ContactCell} />
  `);
  const { reports, context } = reporting();
  // The context a cell materializes with carries the binder back, the way `DrawnTree` builds it.
  const cellContext = { program, ...context };
  cellContext.bindTemplate = (_property, record, params) => templateFactory(program, record, params, cellContext);
  const cell = templateFactory(program, root.ItemTemplate, ["Item", "Index"], cellContext)();
  cell.ContextIndex = 0;
  cell.BindingContext = root.ItemsSource[0];
  assert.equal(cell.Views.length, 1);
  assert.equal(typeof cell.Views[0].ItemTemplate, "function", "the nested list is still templated");
  assert.ok(cell.Views[0].ItemTemplate() instanceof NxTemplateCell);
  assert.deepEqual(reports.failures, []);
  assert.deepEqual(reports.unknown, []);
});
