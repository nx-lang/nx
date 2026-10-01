//! The NX sources `runtime/typescript/test/emitted-ir.test.mjs` runs through the TypeScript
//! runtime, for the differential test to run through the Rust one. Keep the two lists in step.

pub(crate) const SOURCES: &[&str] = &[
    r##"
let answer(): int = { 41 }
let root(): int = { answer() + 1 + (7 / 2) + (7 % 2) }
"##,
    r##"
let root(): int = { 1 / 0 }
"##,
    r##"
external component <Item label:string />
external component <Stack content Children:Item+ />
let root() = { <Stack><Item label="only" /></Stack> }
"##,
    r##"
type Shadow = { Y:float64 = 0.0 }
external component <Shape shadows?:Shadow+ sizes?:float64+ />
let root() = { <Shape shadows={ <Shadow Y=6.0 /> } sizes={3.0} /> }
"##,
    r##"
type User = { name:string = "anon" email?:string }
let root(): User.Update = { <User.Update email={} /> }
"##,
    r##"
abstract type Base = { name:string = "anon" }
type User extends Base = { role:string }
let root() = { <User role="admin" /> }
"##,
    r##"
abstract type EventBase = { source:string = "app" }
type UiEvent extends EventBase =
  | clicked { x:int }
  | dismissed
let root(): UiEvent = { <UiEvent.clicked x=3 /> }
"##,
    r##"
abstract type Base = { name:string }
type User extends Base = { role:string }
external component <Card owner:Base />
let root() = { <Card owner={<User name="Ada" role="admin" />} /> }
"##,
    r##"
abstract type Shape = { name:string = "anon" }
type Figure extends Shape =
  | circle { r:int }
  | square { s:int }
external component <Frame held:Shape />
let root() = { <Frame held={<Figure.circle r=2 />} /> }
"##,
    r##"
type Ints = int+
type AlsoInts = Ints
abstract external component <Item />
external component <Leaf extends Item />
type Items = Item+
external component <Box xs?:AlsoInts content items?:Items />
let root() = { <Box xs={3}><Leaf /></Box> }
"##,
    r##"
type Thickness = { Left:float64 = 0.0  Top:float64 = 0.0 }
abstract external component <Control Padding:Thickness = {<Thickness />} content Children?:Control+ />
external component <Panel extends Control />
let root() = { <Panel Padding={<Thickness Left=4.0 />}><Panel /></Panel> }
"##,
    r##"
external component <TextInput value:string />
component <SearchBox placeholder:string = "Find docs" /> = {
  state { query:string = { placeholder } }
  <TextInput value={query} />
}
let root() = { <SearchBox /> }
"##,
    r##"
type Range = { T:type start:T end:T }
let ints(): <Range T=int/> = { <Range T=int start={1} end={5} /> }
let first(): int = { ints().start }
let moved(): <Range T=int/> = { apply(ints(), <Range.Update T=int end={9} />) }
let root() = { moved() }
"##,
    r##"
type User = { name:string email?:string age?:int }
let key(): User.Property = { User.Property.email }
let root(): User = { apply(<User name="Ada" email="x@y" />, <User.Update email={} />) }
let keys(): User.Property* = { changed(<User.Update age={} name="Ada" />) }
"##,
    r##"
external component <Button label:string emits { Tapped { } } />
component <Counter step:int = 1 /> = {
  state { count:int = 0 }
  <Button label="Add" onTapped=<Update count={count + step} /> />
}
"##,
    r##"
type Contact = { name:string }
let <ContactRow Item:Contact Index:int />: string = {Item.name + "#" + Index}
let <Compact Item:Contact />: string = {Item.name}
external component <List TItem:type ItemsSource?:TItem+ ItemTemplate?:<function Item:TItem Index:int />: string />
component <Section Item:Contact Row:<function Item:Contact Index:int />: string /> = { <Row Item={Item} Index=2 /> }
let root() = <List TItem=Contact ItemsSource={ <Contact name="Ada" /> } ItemTemplate={ContactRow} />
"##,
    r##"
external component <Box Label?:string Same?:boolean Other?:boolean />
let <Wrap Item:object />: string = "w"
let <Plain Item:object />: string = "p"
let F: <function Item:object />: string = {Wrap}
let G: <function Item:object />: string = {Wrap}
let H: <function Item:object />: string = {Plain}
let root() = <Box Label=<F Item="x" /> Same={F == G} Other={F == H} />
"##,
    r##"
type Item = { n:int }
external component <Stack content Children:Item+ />
let xs = { 1 2 3 }
let <Items content Items:Item+ />: Item+ = {Items}
let <Shift Items:Item+ By:int />: Item+ = { for i in Items { <Item n={i.n + By} /> } }
let seed = { for x in xs { <Item n={x} /> } }
let viaComponent() = <Stack> for x in xs { <Item n={x} /> } for x in xs { <Item n={x + 10} /> } </Stack>
let viaFunction() = <Items> <Shift Items={seed} By=0 /> <Shift Items={seed} By=10 /> </Items>
let root() = { viaComponent() viaFunction() }
"##,
    r##"
let xs:string+ = {"a" "b"}
let ys:string+ = {"c"}
let root(): string+ = { xs ys }
"##,
    r##"
type Row = { cells:int+ }
let rows:Row+ = { <Row cells={1 2}/> <Row cells={3 4}/> }
let flat(): int+ = { for r in rows { r.cells } }
let evens(): int* = { for n in 1..=4 { if (n % 2 == 0) { n } } }
let root() = { flat() evens() }
"##,
    r##"
type A = { n:int = 1 }
type Box = { content items:A+ }
let c = false
let root(): Box = { <Box><A/>{if c { <A/> }}</Box> }
"##,
    r##"
type Badge = { n:int = 1 }
type Row = { cells:int+ }
type Box = { content items:Badge+ }
type Result = { spliced:Badge+ values:string+ flat:int+ boxed:Box }
let some:Badge+ = { <Badge/> <Badge/> }
let xs:string+ = {"a" "b"}
let rows:Row+ = { <Row cells={1 2}/> <Row cells={3 4}/> }
let c = false
let root(): Result = <Result
  spliced={some <Badge/>}
  values={ xs "c" }
  flat={for r in rows { r.cells }}
  boxed={<Box><Badge/>{if c { <Badge/> }}</Box>}
/>
"##,
    r##"
type Badge = { n:int = 1 }
type Box = { content items:Badge+ }
type Loose = { content items?:Badge+ }
type Nested = { openInOpen:Box takenInner:Box closedOverOpen:Loose }
let yes = true
let no = false
let root(): Nested = <Nested
  openInOpen={<Box><Badge/>{if yes { if no { <Badge n=2 /> } }}</Box>}
  takenInner={<Box><Badge/>{if yes { if yes { <Badge n=5 /> } }}</Box>}
  closedOverOpen={<Loose><Badge/><Badge n=6 />{if yes { if no { <Badge n=3 /> } } else { <Badge n=4 /> }}</Loose>}
/>
"##,
    r##"
type Badge = { n:int = 1 }
type Box = { content items:Badge+ = { <Badge n=9 /> } }
type Open = { content items?:Badge+ }
type Defaults = { untaken:Open empty:Open noIterations:Open absent:Box }
let c = false
let none:Badge* = { }
let root(): Defaults = <Defaults
  untaken={<Open>{if c { <Badge n=2 /> }}</Open>}
  empty={<Open>{}</Open>}
  noIterations={<Open>{for b in none { b }}</Open>}
  absent={<Box/>}
/>
"##,
    r##"
type Badge = { n:int = 1 }
type Box = { content items:Badge+ }
type Implicit = { emptyArm:Box missingArm:Box alone?:int beside:int+ taken?:int }
let c = false
let t = true
let root(): Implicit = <Implicit
  emptyArm={<Box><Badge/>{if c { <Badge n=2 /> } else { }}</Box>}
  missingArm={<Box><Badge/>{if c { <Badge n=2 /> }}</Box>}
  alone={if c { 1 }}
  beside={if c { 1 } 2}
  taken={if t { 1 }}
/>
"##,
    r##"
type Out = { counted?:int tagged?:string mixed:int+ }
let c = true
let v = { if c { 1 } }
let tags:string? = { if c { "new" } }
let xs:int+ = {5 6}
let either = { if c { 1 } else { xs } }
let root(): Out = <Out
  counted={for x in v { x * 10 }}
  tagged={for t in tags { t + "!" }}
  mixed={for x in either { x }}
/>
"##,
    r##"
type Out = { absent?:string+ present?:string+ reversed?:string+ }
let no = false
let yes = true
let xs:string+ = {"a" "b"}
let none:string* = {}
let root(): Out = <Out
  absent={if no { xs }}
  present={if yes { xs }}
  reversed={if yes { none } else { xs }}
/>
"##,
    r##"
type A = { n:int = 1 }
type Box = { content items?:A+ }
let c = false
let as2:A+ = { <A/> <A n=2 /> }
let root() = { <Box>{if c { as2 }}</Box> }
"##,
    r##"
type A = { n:int = 1 }
type Box = { content items?:A+ }
let root() = { <Box><A/></Box> }
"##,
];
