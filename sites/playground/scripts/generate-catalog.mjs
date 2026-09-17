/**
 * Generates the NX catalog (`catalog/skia.nx`) from the vendored DrawnUI TypeScript sources.
 *
 * The catalog is derived rather than hand-written for two reasons. It is large — roughly twenty
 * controls over a base carrying about fifty properties — and it is not readable off the field
 * declarations: `SkiaLabel.Text`, `FontSize` and most of the rest are getter/setter pairs over
 * private fields, so only a type checker sees them. The generator therefore resolves the same
 * `PropsOf<T>` that DrawnUI's own JSX layer uses, inheriting DrawnUI's curated judgment about which
 * members are author-settable instead of re-deriving it.
 *
 * Usage: npm run generate-catalog
 */
import ts from "typescript";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const probePath = join(appRoot, "src/__catalog_probe__.ts");

/**
 * The root of the catalog's own hierarchy, standing for "anything the reconciler can mount".
 *
 * DrawnUI has no such type: `TextSpan` is not a `SkiaControl`, yet it is a legal child of
 * `SkiaLabel`. Content properties are typed as a list of this, so both fit without granting
 * `TextSpan` fifty properties it does not have.
 */
const NODE_ROOT = "DrawnNode";

/** The one content property name; the reconciler mounts children through it. */
const CONTENT_PROPERTY = "Children";

/**
 * Types resolved to a single NX type regardless of their TypeScript shape.
 *
 * `GridLength` is `number | "Auto" | "*" | \`${number}*\`` upstream and should eventually be a
 * discriminated union; a string carries every spelling the demos use. `Color` is already a string
 * alias upstream and is named here only so the divergence report stays honest.
 */
const FORCED = new Map([
  ["GridLength", "string"],
  ["Color", "string"],
]);

function readRegistryTags() {
  const file = join(appRoot, "src/drawnui/react/reconciler.ts");
  const source = ts.createSourceFile(file, readFileSync(file, "utf8"), ts.ScriptTarget.Latest, true);
  const tags = [];
  const visit = (node) => {
    if (
      ts.isVariableDeclaration(node) &&
      ts.isIdentifier(node.name) &&
      node.name.text === "Registry" &&
      node.initializer &&
      ts.isObjectLiteralExpression(node.initializer)
    ) {
      for (const property of node.initializer.properties) {
        if (ts.isShorthandPropertyAssignment(property)) {
          tags.push({ tag: property.name.text, className: property.name.text });
        } else if (ts.isPropertyAssignment(property) && ts.isIdentifier(property.initializer)) {
          tags.push({ tag: property.name.getText(source), className: property.initializer.text });
        }
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(source);
  if (tags.length === 0) {
    throw new Error("Could not read Registry from the vendored reconciler.");
  }
  return tags;
}

function writeProbe(tags) {
  writeFileSync(
    probePath,
    `import type * as R from "./drawnui/react/index";
import type * as C from "./drawnui/index";
type P<T> = T extends (props: infer Q) => unknown ? Q : never;
${tags.map(({ tag }) => `export declare const props_${tag}: P<typeof R.${tag}>;`).join("\n")}
${tags.map(({ tag, className }) => `export declare const inst_${tag}: C.${className};`).join("\n")}
`,
  );
}

function createProgram() {
  const config = ts.readConfigFile(join(appRoot, "tsconfig.json"), ts.sys.readFile);
  const parsed = ts.parseJsonConfigFileContent(config.config, ts.sys, appRoot);
  const program = ts.createProgram([probePath], parsed.options);
  return { program, checker: program.getTypeChecker() };
}

/** The catalog is written in one pass; these accumulate what the emitters need. */
const unions = new Map();
const records = new Map();
const omissions = new Map();

/** A property is omitted once, however many controls inherit it. */
function omit(owner, property, reason) {
  omissions.set(`${owner}.${property}`, { owner, property, reason });
}

/** An event parameter with no NX type is dropped from the emit's payload, and recorded like a property. */
function dropParameter(owner, event, parameter, reason) {
  omissions.set(`${owner}.${event}(${parameter})`, { owner, property: event, parameter, reason });
}

/** Events by declaring class: each an emit name, its payload fields, and the parameter names in order. */
const eventsByClass = new Map();
const synthesizedNames = new Map();
const overrides = [];

function stripUndefined(checker, type) {
  if (!type.isUnion()) {
    return type;
  }
  const kept = type.types.filter((member) => !(member.flags & (ts.TypeFlags.Undefined | ts.TypeFlags.Null)));
  return kept.length === 1 ? kept[0] : checker.getUnionType(kept);
}

function aliasName(type) {
  return type.aliasSymbol?.name;
}

/**
 * The type name as written on the declaration, when it is a bare reference.
 *
 * `Partial<T>` loses the alias a property was declared with, so `HorizontalOptions: LayoutOptions`
 * arrives as an anonymous union of four string literals. Reading the annotation back off the
 * declaration recovers DrawnUI's own name for it, which is the name a playground author already knows
 * from the C# and TypeScript APIs.
 */
function annotationName(declaration) {
  const node = ts.isSetAccessorDeclaration(declaration)
    ? declaration.parameters[0]?.type
    : declaration.type;
  return node !== undefined && ts.isTypeReferenceNode(node) && ts.isIdentifier(node.typeName)
    ? node.typeName.text
    : undefined;
}

/** Both halves of an accessor pair carry names; the annotated one wins. */
function declaredName(property) {
  for (const declaration of property.declarations ?? []) {
    const name = annotationName(declaration);
    if (name !== undefined) {
      return name;
    }
  }
  return undefined;
}

/**
 * The one call signature of an event's function type: one returning `void`, or a union that
 * includes it (`ContextMenu` returns `boolean | void`). Any other function member is not an
 * event and has no NX expression.
 */
function eventSignature(checker, type) {
  const signatures = checker.getSignaturesOfType(type, ts.SignatureKind.Call);
  if (signatures.length !== 1) {
    return null;
  }
  const returned = signatures[0].getReturnType();
  const isVoid = (candidate) => (candidate.flags & ts.TypeFlags.Void) !== 0;
  return isVoid(returned) || (returned.isUnion() && returned.types.some(isVoid)) ? signatures[0] : null;
}

/** The NX primitive a callback parameter's type maps to, or null: an emit's payload carries nothing richer. */
function primitiveNx(type) {
  if (type.flags & ts.TypeFlags.String) {
    return "string";
  }
  if (type.flags & (ts.TypeFlags.Number | ts.TypeFlags.NumberLiteral)) {
    return "float64";
  }
  if (type.flags & (ts.TypeFlags.Boolean | ts.TypeFlags.BooleanLiteral)) {
    return "boolean";
  }
  return null;
}

/**
 * Reads an event off its signature: the parameters after the leading sender become payload fields
 * under their own names where their type is a primitive, and are dropped and recorded otherwise.
 */
function readEvent(checker, owner, name, signature) {
  const fields = [];
  const params = [];
  let dropped = false;
  for (const parameter of signature.parameters.slice(1)) {
    const type = stripUndefined(checker, checker.getTypeOfSymbolAtLocation(parameter, parameter.valueDeclaration));
    const nx = primitiveNx(type);
    if (nx === null) {
      dropParameter(owner, name, parameter.name, checker.typeToString(type));
      dropped = true;
      continue;
    }
    // The renderer fills payload fields from the callback's arguments by position, so a kept
    // parameter after a dropped one would be filled from the dropped one's argument.
    if (dropped) {
      throw new Error(`${owner}.${name}: a dropped parameter precedes a kept one, which the metadata cannot express.`);
    }
    fields.push({ name: parameter.name, nx });
    params.push(parameter.name);
  }
  return { name, fields, params };
}

function isStringLiteralUnion(type) {
  return type.isUnion() && type.types.every((member) => member.flags & ts.TypeFlags.StringLiteral);
}

function declaredIn(symbol, fragment) {
  return (symbol?.declarations ?? []).some((declaration) =>
    declaration.getSourceFile().fileName.includes(fragment),
  );
}

function recordShape(checker, type, name) {
  if (records.has(name)) {
    return records.get(name);
  }
  const entry = { name, fields: [], construct: null };
  records.set(name, entry);
  for (const property of checker.getPropertiesOfType(type)) {
    const declaration = property.declarations?.[0];
    if (declaration === undefined) {
      continue;
    }
    // Getters without setters are derived (`Thickness.HorizontalThickness`), and statics are not
    // per-value data. Neither is something an author supplies.
    if (ts.isGetAccessorDeclaration(declaration) || ts.isMethodDeclaration(declaration)) {
      continue;
    }
    const propertyType = stripUndefined(checker, checker.getTypeOfSymbolAtLocation(property, declaration));
    const mapped = mapType(checker, propertyType, `${name}${property.name}`, declaredName(property));
    if (mapped === null) {
      omit(name, property.name, checker.typeToString(propertyType));
      continue;
    }
    entry.fields.push({ name: property.name, nx: mapped.nx });
  }
  entry.construct = declaredIn(type.symbol, "core/Types") && type.symbol.valueDeclaration !== undefined
    ? name
    : null;
  return entry;
}

function unionShape(name, cases) {
  const existing = unions.get(name);
  if (existing !== undefined) {
    return existing;
  }
  const entry = { name, cases };
  unions.set(name, entry);
  return entry;
}

function synthesizeName(context) {
  const name = context.replace(/[^A-Za-z0-9]/g, "");
  synthesizedNames.set(name, context);
  return name;
}

/**
 * Maps a resolved TypeScript type onto an NX type, or returns null where the property has no NX
 * expression yet. Every null is reported, so the catalog's divergences are enumerated rather than
 * discovered later.
 */
function mapType(checker, type, context, preferred) {
  const alias = preferred ?? aliasName(type);
  if (alias !== undefined && FORCED.has(alias)) {
    return { nx: FORCED.get(alias), meta: { kind: "primitive" } };
  }
  if (type.flags & ts.TypeFlags.String) {
    return { nx: "string", meta: { kind: "primitive" } };
  }
  if (type.flags & (ts.TypeFlags.Number | ts.TypeFlags.NumberLiteral)) {
    return { nx: "float64", meta: { kind: "primitive" } };
  }
  if (type.flags & (ts.TypeFlags.Boolean | ts.TypeFlags.BooleanLiteral)) {
    return { nx: "boolean", meta: { kind: "primitive" } };
  }
  if (checker.getSignaturesOfType(type, ts.SignatureKind.Call).length > 0) {
    return null;
  }
  if (type.flags & (ts.TypeFlags.Unknown | ts.TypeFlags.Any)) {
    return null;
  }

  if (checker.isArrayType(type) || checker.isTupleType(type) || (type.symbol?.name === "ReadonlyArray")) {
    const element = stripUndefined(checker, checker.getTypeArguments(type)[0]);
    if (element === undefined) {
      return null;
    }
    const mapped = mapType(checker, element, `${context}[]`, preferred);
    return mapped === null ? null : { nx: `${mapped.nx}[]`, meta: { kind: "list", element: mapped.meta } };
  }

  if (type.flags & ts.TypeFlags.StringLiteral) {
    // A lone literal (`SkiaGradient.Type: "Linear"`) is a union of one; NX spells that with a bar.
    const name = alias ?? synthesizeName(context);
    unionShape(name, [type.value]);
    return { nx: name, meta: { kind: "union", name } };
  }

  if (type.isUnion()) {
    if (isStringLiteralUnion(type)) {
      const cases = type.types.map((member) => member.value);
      if (!cases.every((name) => /^[A-Za-z_][A-Za-z0-9_]*$/.test(name))) {
        return null;
      }
      const name = alias ?? synthesizeName(context);
      unionShape(name, cases);
      return { nx: name, meta: { kind: "union", name } };
    }

    // A union that mixes shapes has no NX spelling, so it collapses to its single richest member:
    // the record where one is present, `string` where the members are literal spellings of one.
    const richest = type.types.find((member) => {
      const cleaned = stripUndefined(checker, member);
      return cleaned.symbol !== undefined && declaredIn(cleaned.symbol, "core/Types");
    });
    if (richest !== undefined) {
      return mapType(checker, stripUndefined(checker, richest), context, preferred);
    }
    const arrayMember = type.types.find((member) => checker.isArrayType(member));
    if (arrayMember !== undefined && type.types.some((member) => member.flags & ts.TypeFlags.String)) {
      // `string | GridLength[]`: every demo writes the string spelling.
      return { nx: "string", meta: { kind: "primitive" } };
    }
    if (type.types.every((member) => member.flags & (ts.TypeFlags.String | ts.TypeFlags.StringLiteral | ts.TypeFlags.Number | ts.TypeFlags.TemplateLiteral))) {
      return { nx: "string", meta: { kind: "primitive" } };
    }
    return null;
  }

  // A record is one of DrawnUI's value types, which all live in `core/Types`. Any other class —
  // a control, an effect, the canvas — is an engine object an author cannot build, and following
  // it would walk the whole engine into the catalog (`VisualEffects: SkiaEffect[]` reaches
  // `SkiaControl` through `Parent`). Those properties are omitted and listed like callbacks.
  if (type.symbol !== undefined && declaredIn(type.symbol, "core/Types")) {
    const name = type.symbol.name;
    // `Partial<SkiaShadow>` and friends resolve to the same shape under a mapped-type name.
    const cleanName = name.startsWith("Partial<") ? name.slice(8, -1) : name;
    recordShape(checker, type, cleanName);
    return { nx: cleanName, meta: { kind: "record", name: cleanName } };
  }

  return null;
}

function ownerOf(property) {
  const declaration = property.declarations?.[0];
  if (declaration === undefined) {
    return null;
  }
  const parent = declaration.parent;
  return ts.isClassDeclaration(parent) && parent.name !== undefined ? parent.name.text : null;
}

function classChain(checker, tag, exportsByName) {
  const symbol = exportsByName.get(`inst_${tag}`);
  const type = checker.getTypeOfSymbolAtLocation(symbol, symbol.valueDeclaration);
  const chain = [];
  let declaration = type.symbol?.declarations?.find(ts.isClassDeclaration);
  while (declaration !== undefined) {
    chain.push(declaration.name.text);
    const base = ts.getEffectiveBaseTypeNode(declaration);
    if (base === undefined) {
      break;
    }
    const baseSymbol = checker.getSymbolAtLocation(base.expression);
    const resolved = baseSymbol?.flags & ts.SymbolFlags.Alias ? checker.getAliasedSymbol(baseSymbol) : baseSymbol;
    declaration = resolved?.declarations?.find(ts.isClassDeclaration);
  }
  return chain;
}

function nxUnionDeclaration(union) {
  if (union.cases.length === 1) {
    return `export type ${union.name} = | ${union.cases[0]}\n`;
  }
  if (union.cases.length <= 5) {
    return `export type ${union.name} = ${union.cases.join(" | ")}\n`;
  }
  return `export type ${union.name} =\n${union.cases.map((name) => `  | ${name}`).join("\n")}\n`;
}

function nxRecordDeclaration(record) {
  const fields = record.fields.map((field) => `  ${field.name}: ${field.nx}?`).join("\n");
  return `export type ${record.name} = {\n${fields}\n}\n`;
}

function nxEmit(event) {
  const fields = event.fields.map((field) => `${field.name}:${field.nx}`).join(" ");
  return `    ${event.name} { ${fields}${fields.length > 0 ? " " : ""}}`;
}

function nxComponent({ name, isAbstract, base, props, hasContent, emits = [] }) {
  // Exported, because the catalog is its own module: the playground and the language service
  // reach it through an implicit import, which sees only what a module exports.
  const header = `export ${isAbstract ? "abstract " : ""}external component`;
  const lines = props.map((prop) => `  ${prop.name}: ${prop.nx}?`);
  if (hasContent) {
    lines.push(`  content ${CONTENT_PROPERTY}: ${NODE_ROOT}[]?`);
  }
  if (emits.length > 0) {
    // Each emit declares its own record, so the action a handler receives is `<Component>.<Event>`.
    lines.push("  emits {", ...emits.map(nxEmit), "  }");
  }
  const extendsClause = base === null ? "" : ` extends ${base}`;
  if (lines.length === 0) {
    return `${header} <${name}${extendsClause} />\n`;
  }
  return `${header}\n<${name}${extendsClause}\n${lines.join("\n")}\n/>\n`;
}

function main() {
  const tags = readRegistryTags();
  writeProbe(tags);
  try {
    const { program, checker } = createProgram();
    const source = program.getSourceFile(probePath);
    const moduleSymbol = checker.getSymbolAtLocation(source);
    const exportsByName = new Map(
      checker.getExportsOfModule(moduleSymbol).map((symbol) => [symbol.name, symbol]),
    );

    const byClass = new Map();
    const tagInfo = new Map();

    for (const { tag } of tags) {
      const chain = classChain(checker, tag, exportsByName);
      const symbol = exportsByName.get(`props_${tag}`);
      const propsType = checker.getTypeOfSymbolAtLocation(symbol, symbol.valueDeclaration);
      let hasContent = false;
      for (const property of checker.getPropertiesOfType(propsType)) {
        if (property.name === "ref") {
          continue;
        }
        if (property.name === "children") {
          hasContent = true;
          continue;
        }
        const owner = ownerOf(property);
        if (owner === null) {
          continue;
        }
        const declaration = property.declarations[0];
        const type = stripUndefined(checker, checker.getTypeOfSymbolAtLocation(property, declaration));
        // An event is an optional member: a required function member is something the control calls.
        // The props type wraps every member in `Partial<>`, so the symbol is always optional; the
        // class member's own declaration says whether DrawnUI declared it optional.
        const optional = declaration.questionToken !== undefined;
        const signature = optional ? eventSignature(checker, type) : null;
        if (signature !== null) {
          if (!eventsByClass.has(owner)) {
            eventsByClass.set(owner, new Map());
          }
          if (!eventsByClass.get(owner).has(property.name)) {
            eventsByClass.get(owner).set(property.name, readEvent(checker, owner, property.name, signature));
          }
          continue;
        }
        const mapped = mapType(checker, type, `${owner}${property.name}`, declaredName(property));
        if (mapped === null) {
          omit(owner, property.name, checker.typeToString(type));
          continue;
        }
        if (!byClass.has(owner)) {
          byClass.set(owner, new Map());
        }
        byClass.get(owner).set(property.name, mapped);
      }
      tagInfo.set(tag, { chain, hasContent });
    }

    // TypeScript lets a subclass restate an inherited member — `SkiaShape.Type` narrows
    // `SkiaLayout.Type`, `SkiaRichLabel.FontSize` restates `SkiaLabel.FontSize`. NX rejects a
    // redeclared inherited prop, and the ancestor's declaration is the wider of the two, so the
    // restatement is dropped and recorded.
    const parentOf = new Map();
    for (const { chain } of tagInfo.values()) {
      for (let index = 0; index + 1 < chain.length; index += 1) {
        parentOf.set(chain[index], chain[index + 1]);
      }
    }
    for (const members of [byClass, eventsByClass]) {
      for (const [className, own] of members) {
        for (let ancestor = parentOf.get(className); ancestor !== undefined; ancestor = parentOf.get(ancestor)) {
          for (const name of members.get(ancestor)?.keys() ?? []) {
            if (own.has(name)) {
              overrides.push({ owner: className, property: name, inheritedFrom: ancestor });
              own.delete(name);
              // A folded event's dropped parameters are the ancestor's to record, not this class's.
              for (const [key, omission] of omissions) {
                if (omission.owner === className && omission.property === name && omission.parameter !== undefined) {
                  omissions.delete(key);
                }
              }
            }
          }
        }
      }
    }
    const emitsOf = (className) =>
      [...(eventsByClass.get(className) ?? new Map()).values()].sort((left, right) => left.name.localeCompare(right.name));

    // A class that other registered tags extend needs an abstract twin, since only abstract
    // components may be extended.
    const registered = new Set(tags.map(({ tag }) => tag));
    const ancestors = new Set();
    for (const { chain } of tagInfo.values()) {
      for (const className of chain.slice(1)) {
        ancestors.add(className);
      }
    }
    const abstractNameFor = (className) =>
      registered.has(className) ? `${className}Base` : className;

    const declarations = [];
    const meta = { contentProperty: CONTENT_PROPERTY, nodeRoot: NODE_ROOT, unions: {}, records: {}, components: {} };

    declarations.push(nxComponent({ name: NODE_ROOT, isAbstract: true, base: null, props: [], hasContent: false }));

    const emittedAbstracts = new Set();
    const emitAbstract = (className, chain) => {
      if (emittedAbstracts.has(className)) {
        return;
      }
      emittedAbstracts.add(className);
      const index = chain.indexOf(className);
      const parent = chain[index + 1];
      const base = parent === undefined ? NODE_ROOT : abstractNameFor(parent);
      if (parent !== undefined) {
        emitAbstract(parent, chain);
      }
      const props = [...(byClass.get(className) ?? new Map()).entries()]
        .map(([name, mapped]) => ({ name, nx: mapped.nx }))
        .sort((left, right) => left.name.localeCompare(right.name));
      declarations.push(
        nxComponent({ name: abstractNameFor(className), isAbstract: true, base, props, hasContent: false, emits: emitsOf(className) }),
      );
    };

    for (const [tag, { chain }] of tagInfo) {
      for (const className of chain) {
        if (ancestors.has(className)) {
          emitAbstract(className, chain);
        }
      }
      void tag;
    }

    for (const { tag } of tags) {
      const { chain, hasContent } = tagInfo.get(tag);
      const ownClass = chain[0];
      const isAlsoBase = ancestors.has(ownClass);
      const base = isAlsoBase
        ? abstractNameFor(ownClass)
        : chain[1] === undefined
          ? NODE_ROOT
          : abstractNameFor(chain[1]);
      const props = isAlsoBase
        ? []
        : [...(byClass.get(ownClass) ?? new Map()).entries()]
            .map(([name, mapped]) => ({ name, nx: mapped.nx }))
            .sort((left, right) => left.name.localeCompare(right.name));
      const emits = isAlsoBase ? [] : emitsOf(ownClass);
      declarations.push(nxComponent({ name: tag, isAbstract: false, base, props, hasContent, emits }));
      // Every event the control carries, inherited included, with the parameter names the
      // renderer builds the action from; the action's name comes from the prepared declaration.
      const events = {};
      for (const className of [...chain].reverse()) {
        for (const event of emitsOf(className)) {
          events[event.name] = event.params;
        }
      }
      meta.components[tag] = {
        class: ownClass,
        content: hasContent ? CONTENT_PROPERTY : null,
        events: Object.fromEntries(Object.entries(events).sort(([left], [right]) => left.localeCompare(right))),
      };
    }

    for (const [name, union] of [...unions.entries()].sort()) {
      meta.unions[name] = union.cases;
    }
    for (const [name, record] of [...records.entries()].sort()) {
      meta.records[name] = { construct: record.construct, fields: record.fields.map((field) => field.name) };
    }

    const header = `// Generated by scripts/generate-catalog.mjs from the vendored DrawnUI sources.
// Do not edit by hand: run \`npm run generate-catalog\` instead.
//
// Every property is optional. DrawnUI's own defaults are the defaults: the renderer drops nulls,
// so an unset property is left for the control to decide rather than restated here, and the
// catalog cannot drift from the vendored code it was generated against.
`;

    const body = [
      header,
      ...[...unions.values()].sort((a, b) => a.name.localeCompare(b.name)).map(nxUnionDeclaration),
      ...[...records.values()].sort((a, b) => a.name.localeCompare(b.name)).map(nxRecordDeclaration),
      ...declarations,
    ].join("\n");

    mkdirSync(join(appRoot, "catalog"), { recursive: true });
    writeFileSync(join(appRoot, "catalog/skia.nx"), body);
    writeFileSync(join(appRoot, "catalog/catalog-meta.json"), `${JSON.stringify(meta, null, 2)}\n`);
    writeFileSync(
      join(appRoot, "catalog/overrides.json"),
      `${JSON.stringify(
        overrides.sort((a, b) => `${a.owner}.${a.property}`.localeCompare(`${b.owner}.${b.property}`)),
        null,
        2,
      )}\n`,
    );
    writeFileSync(
      join(appRoot, "catalog/omitted.json"),
      `${JSON.stringify(
        [...omissions.values()].sort((a, b) =>
          `${a.owner}.${a.property}.${a.parameter ?? ""}`.localeCompare(`${b.owner}.${b.property}.${b.parameter ?? ""}`),
        ),
        null,
        2,
      )}\n`,
    );

    const emitCount = [...eventsByClass.values()].reduce((total, events) => total + events.size, 0);
    console.log(
      `catalog: ${Object.keys(meta.components).length} components, ${unions.size} unions, ${records.size} records, ${emitCount} emits, ${omissions.size} members omitted, ${overrides.length} overrides folded into their base`,
    );
  } finally {
    rmSync(probePath, { force: true });
  }
}

main();
