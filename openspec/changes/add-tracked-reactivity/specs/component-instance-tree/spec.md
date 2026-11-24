## ADDED Requirements

### Requirement: A node handed a new descriptor renders again only when a prop it read changed
A node's render SHALL record the props it read: a prop whose `slot` node the body evaluated, a prop
captured by an override formula the body wrote, and a prop that a handler the body bound reads,
found as a formula's captured locals are found. A content parameter SHALL count as one prop, and
handler props bound for emits SHALL NOT count as reads. When a visit hands a node a descriptor of
its component under its parent that is not equal to the one it holds, and the node's environment
record holds at its new place, the tree SHALL normalize the new descriptor's props, charged as
initialization charges normalizing them, and SHALL compare the recorded props of the old and the new
normalized props as two descriptors are compared when a kept child is rebound: a handler the body
can see resolved through the instance each descriptor was read from and compared as `==` compares
it, a handler bound for an emit bound in both or in neither, and the text of a token not counting.
The comparison SHALL count against the visit's operation budget as the comparison for a rebinding
does. When a recorded prop differs, the node SHALL be initialized again from the props normalized
for the comparison, so that normalization is charged once. Otherwise the node SHALL keep its render:
it SHALL be rebound as a kept child is, when its parent's instance changed or a handler it holds is
replaced; its instance SHALL take the new normalized props and the new handler props without
rendering, and SHALL keep its generation and tokens; and the node SHALL take the new descriptor and
environment and SHALL stay settled. A node's props SHALL NOT be part of what makes its instance the
same render for the tree's comparisons, so a prop skip that rebinds nothing leaves the nodes
initialized through the node matching it.

#### Scenario: A prop the body ignores does not render
- **WHEN** a `Badge` node's body reads `label` and not `tooltip`, and a pass hands it a descriptor
  whose only change is `tooltip`
- **THEN** the node SHALL keep its rendered output, hold the new descriptor, and stay settled
- **AND** a later dispatch that renders the node SHALL see the new `tooltip`

#### Scenario: A prop a handler reads renders
- **WHEN** a handler that `Badge`'s body binds reads `tooltip`, and a pass hands the node a new
  `tooltip`
- **THEN** the node SHALL render again

#### Scenario: A prop an override formula captures renders
- **WHEN** `Row`'s body renders `<Cell env.gap={env.space * scale} />` with a prop `scale` and reads
  `scale` nowhere else, and a pass hands the `Row` node a new `scale`
- **THEN** the `Row` node SHALL render again, and the `Cell` SHALL read the new gap

#### Scenario: A prop whose default reads a changed prop renders
- **WHEN** a component declares `label: string` and `title: string = label`, its body reads only
  `title`, and a pass hands its node a descriptor that leaves `title` out and changes `label`
- **THEN** the node SHALL render again with the new `title`

#### Scenario: A kept render under a new parent instance is rebound
- **WHEN** `Page` renders `<Stack note={note}><Button onTapped=<State.Update taps={taps + 1} />
  /></Stack>`, `Stack` renders its content and does not read `note`, `Page` renders again with
  another `note`, and a pass visits the `Stack` node
- **THEN** the `Stack` node SHALL keep its rendered output and stay settled
- **AND** a tap on the button through the `Stack` node SHALL patch the `Page` node's state

### Requirement: The tree keeps derived results across renders
The tree SHALL keep the top-level values, defaults and entry-module provides its renders evaluated,
when no read they recorded, at any depth, was supplied or capped by a layer the render pushed, and
SHALL offer them to later renders of any node. Formulas, and the bundles of a render's own
`envProvide` nodes, SHALL be reused within one call only. The tree SHALL pass its kept results to a
render through an internal option, not a public API. Each variant SHALL have, beside its direct
inputs, the flattened set of the environment values its leaf reads read at any depth, through its
edges included, found from its direct inputs and edges when it is needed. A render SHALL take a variant from the tree only when no
layer the render pushed touches a value in that set, and the variant SHALL enter the call's own
reuse as if evaluated at the call's start, with that set's reads and their tokens taken at the use.
Validation SHALL use the direct inputs only.

The tree SHALL keep one revision counter, which each `set_environment` advances once. Each root
value it sets, changes or clears SHALL take the new revision, a value changing when it is not the
same as before as a host would see it written, by the cutoff's rule below, and each kept result whose value
changes when it is evaluated again SHALL take the current revision. Each variant of a kept result SHALL hold the revision at which its value last changed, the
revision at which it was last found valid, and its record of direct inputs. A variant SHALL be valid
at a use when its inputs, checked in the order they were read, all hold there: each read classified
at the use as it was recorded, a root read whose root value has not changed since the variant was
last found valid, an edge whose variant, validated first at the use by this rule and evaluated again
there with the cutoff below when it is not valid, has changed no later than the record was last
found valid, and an override-supplied read whose value the environment lookup gives at the use is
the same as a host would see it written, by the cutoff's rule below. The lookup SHALL evaluate any formula there and SHALL be charged as the skip check's reads
and comparisons are. An evaluation of the used result that gives another variant than the edge names
SHALL count as a change. The check SHALL stop at the first input that does not hold, and the result
SHALL then be evaluated again without checking the rest, so that the check never evaluates what a
fresh evaluation would not. A lookup or an evaluation that fails during the check, for any reason
but a resource limit, SHALL count as an input that does not hold, so that the evaluation that
follows raises the failure where a fresh evaluation would; a resource limit SHALL end the
operation. Classifying a read and comparing revisions SHALL cost no operations.

A result evaluated again SHALL be compared with the variant it replaces, the first variant of the
result whose reads are classified at the place as they were recorded and that was found not valid
there, charged as a comparison of values. For a variant whose check stopped at an input that
changed, the reads after it SHALL be classified too, at no cost, and one classified otherwise
SHALL make the variant another place's. When the two are the same value as a host would see it written, of the same number kind,
with `-0.0` distinct from `0.0`, a NaN the same as nothing, and, when the host collects origins,
with equal origins, the tree SHALL keep the old value and its revision. A result whose reads are
classified differently, or whose override-supplied values differ, at two places SHALL keep one
variant for each, and a use SHALL take the first valid variant in the order they were kept. A
variant SHALL be dropped when no node's record and no other kept result references it. A kept result
shared by several nodes SHALL be paid for by the render that evaluates it first. The check of a
node's record, with any kept result it evaluates again, SHALL be charged to the lifecycle call that
follows it at the node: that call SHALL start with what the check left of the operation budget and
SHALL report the check's operations with its own. A check that holds with no lifecycle call after
it SHALL be reported as a call of its own, and one that fails SHALL report what it spent. A render's
operation count SHALL be a function of the images, its input and what the tree holds. Running a
change through `atomically` SHALL restore the kept results, the revision counter, the reverse
indexes and the change set with the nodes when it fails.

#### Scenario: A render after a dispatch reuses the palette
- **WHEN** a node whose body reads `env.color.accent` is dispatched against and renders again with
  no change to the environment
- **THEN** the render SHALL reuse `color`'s kept default and `standardPalette`'s kept value
- **AND** it SHALL cost less than the same render in a new tree

#### Scenario: A kept result survives passes that render nothing
- **WHEN** a node is dispatched against and renders, a pass follows that renders nothing, and the
  node is dispatched against and renders again
- **THEN** the second render SHALL reuse `color`'s kept default without evaluating it

#### Scenario: A root change makes kept results that read it stale
- **WHEN** a host changes the root value of `colorMode`, and a node then renders
- **THEN** `color`'s default SHALL be evaluated afresh in the new mode
- **AND** a kept result that read no changed value SHALL be reused without evaluating it

#### Scenario: Light and dark sections keep a variant each
- **WHEN** a pass renders one `Card` node inside `<div env.colorMode=light>` and another inside
  `<div env.colorMode=dark>`, and a later pass renders both again
- **THEN** the later pass SHALL reuse a `color` variant for each section without evaluating either

#### Scenario: A result kept where nothing overrides is not used under an override
- **WHEN** a `Card` node with no override above it renders and keeps `color`'s default, which read
  `env.colorMode` from the root, and a `Card` node under `<div env.colorMode=dark>` then renders
- **THEN** `color`'s default SHALL be evaluated again for the second node, which SHALL read the dark
  colors

#### Scenario: A render that overrides a value a kept result read below an edge evaluates it again
- **WHEN** the tree keeps `button`'s default, which used `color`'s default, which read
  `env.colorMode` from the root, and a node's body reads `env.button` inside its own
  `provide env.colorMode=dark`
- **THEN** the render SHALL evaluate `button`'s and `color`'s defaults again and read the dark
  button

#### Scenario: A used result is evaluated again before its edge is judged
- **WHEN** a node's render used a kept `direction` result that read `env.locale`, and a host
  changes `locale` so that `direction` evaluates again to `ltr`
- **THEN** checking the node's edge SHALL evaluate `direction` again and find it unchanged
- **AND** the node SHALL NOT render again

#### Scenario: An input only a branch no longer taken read is not checked
- **WHEN** a kept result reads `env.flag` and, only when it is `true`, `env.detail`, whose default
  fails once a root value it reads changes, and a host changes `flag` to `false` and that root value
  in one `set_environment`
- **THEN** the result SHALL be evaluated again without evaluating `detail`'s default
- **AND** `set_environment` SHALL succeed

#### Scenario: A failed change leaves the kept results as they were
- **WHEN** a `set_environment` re-evaluates a kept result and then fails in a render
- **THEN** the tree's kept results, revisions and change set SHALL be as they were before the call

#### Scenario: A kept result is not taken where an override supplies a value written otherwise
- **WHEN** a kept result reads `env.z`, and a page renders the same `Card` under `env.z={0.0}` and
  under `env.z={-0.0}`
- **THEN** each card SHALL show the zero its section supplies, as a fresh render does

#### Scenario: An equal result stops a change whichever variant is checked first
- **WHEN** `color` reads `env.locale` and then `env.colorMode`, a light card and a card under
  `env.colorMode=dark` each keep a variant of it, and a host changes `locale` to a value that leaves
  both colors as they were
- **THEN** neither card SHALL render again, in whichever order their keys are checked

#### Scenario: A kept result evaluated again during a check is charged to the render that follows
- **WHEN** a host changes `colorMode`, the check of a `Card` node's record evaluates `color`'s
  default again, and the card renders
- **THEN** the usage the call reports SHALL count `color`'s evaluation, under one operation budget

### Requirement: A change notifies only the computations that read it
The tree SHALL keep, for each environment value, an index of the nodes and kept results with a root
read of it, and of those with an override-supplied read whose lookup read its root value, and, for
each kept result, the nodes and kept results that used it. `set_environment` SHALL follow those
indexes from the values whose root value changed to mark possibly stale exactly the nodes and kept
results they reach, evaluating nothing, and SHALL check nothing else. Then, in order from the nodes
nearest the root, it SHALL check each possibly stale node's record at its environment by the rule on
kept derived results: each kept result the node used validated first, and evaluated again with the
cutoff only when it is not valid, its edge holding when the result's revision of last change is no
later than the node's. It SHALL render the node again at the first direct input that does not hold.
A dispatch SHALL decide whether to render from the dispatched node's own record. The indexes SHALL
stay current as nodes render, are replaced, removed and dropped, and as kept results are replaced
and dropped.

#### Scenario: Unrelated kept results are not checked
- **WHEN** the tree holds kept results that read `colorMode` and others that read only `locale`, and
  a host changes `colorMode`
- **THEN** only the results that read `colorMode`, and what used them, SHALL be marked possibly stale
- **AND** the others SHALL NOT be validated

#### Scenario: A node whose inputs come back equal is left alone
- **WHEN** a node's render read only a kept `direction` result, a host changes `locale`, and
  `direction` evaluates again to an equal value
- **THEN** the node SHALL NOT render again and SHALL stay settled

### Requirement: The tree reports a change set to the host
The tree SHALL keep a change set and `take_changes` SHALL return it and clear it. The set SHALL be
a diff by key between the tree when the host last took it and the tree now. A key with no node then
and a node now SHALL be listed as added, and a key with a node then and none now as removed, every
node dropped with a removed or replaced node being listed under its own key. A key whose node was
replaced in between, by a visit with another component or under another parent, or removed and
then visited again, SHALL be listed as removed and added. A key that holds the same node then and
now SHALL be listed as rendered when the node's rendered output changed in any way since, by a
render or by a rebinding that changed what its tokens name, and SHALL NOT be listed when the node
only took a new descriptor or new state. A node added and removed before the host took the set
SHALL appear in no list. A node added and then rendered again SHALL appear as added only. A node
that rendered several times SHALL appear once. `is_settled` SHALL remain, and a node SHALL be
unsettled exactly when it or a node under it rendered or was added since a pass last walked it.

#### Scenario: A dispatch reports the node it rendered
- **WHEN** a host dispatches against a `Counter` node whose handler patches a field its body reads,
  and takes the change set
- **THEN** the set SHALL list the `Counter` node as rendered, and nothing as added or removed

#### Scenario: A pass reports what it created and dropped
- **WHEN** a pass visits a new `Row` descriptor and no longer finds an old one, and the host takes
  the change set
- **THEN** the set SHALL list the new node as added and the old one as removed

#### Scenario: A dispatch that renders nothing reports nothing
- **WHEN** a dispatch patches only a field no render read, and the host takes the change set
- **THEN** the set SHALL be empty

#### Scenario: A node replaced at its key is removed and added
- **WHEN** a key that held a `Counter` node is visited with a `Timer` descriptor, and the host
  takes the change set
- **THEN** the set SHALL list the key as removed and as added

#### Scenario: A dropped subtree lists every node in it
- **WHEN** a pass drops a `Panel` node that has a `Card` node under it, and the host takes the
  change set
- **THEN** the set SHALL list both the `Panel` and the `Card` keys as removed

#### Scenario: A rebound node is reported as rendered
- **WHEN** a pass keeps and rebinds a `Card` node under a `Page` node that rendered again, and the
  host takes the change set
- **THEN** the set SHALL list the `Card` node as rendered

## MODIFIED Requirements

### Requirement: The Rust runtime exports an instance tree over a linked program
The Rust IR runtime SHALL export an instance tree created for one linked program. The tree SHALL
compose instances only through the runtime's component lifecycle, through two internal lifecycle
entries, one that runs a dispatch's handlers and renders only when the tree needs it and one that
normalizes props without rendering, and through keeping and rebinding a node, which evaluates
nothing, so that every state it yields, and every diagnostic of an evaluation, is one the lifecycle
operations yield for the same props, state and batches, and every rendered output it yields is the
one the lifecycle renders for its props and state, with the tokens of the render that produced it.
Every operation of the tree that evaluates SHALL take runtime options and SHALL pass them to each
lifecycle call it makes; the operation budget and the input limit SHALL apply to each of those calls
separately, a record check sharing the budget of the call that follows it. The tree SHALL say whether a name is a component the program can instantiate, which is
what the lifecycle can initialize: a component entrypoint of the entry module declared with a body.
A host tells by it an authored component in rendered output from a record that is the host's own to
interpret.

#### Scenario: A node holds what the lifecycle renders
- **WHEN** a host visits a descriptor of `Counter` in a tree and separately initializes `Counter`
  from the same props through the lifecycle
- **THEN** the node's rendered output and tokens SHALL equal the initialization's

#### Scenario: An authored component is told from an external one
- **WHEN** a program declares `component <Card /> = { <Label /> }` and `external component <Label />`
- **THEN** the tree SHALL answer that `Card` can be instantiated
- **AND** SHALL answer that `Label` and an undeclared name cannot

#### Scenario: A limit reached inside a tree operation is the runtime's diagnostic
- **WHEN** a host dispatches through a tree under an operation budget a handler exhausts
- **THEN** the operation SHALL fail with `nx-ir-resource-limit` naming the operation budget

### Requirement: Props flow down and state stays with the node
A node SHALL NOT be initialized again when it is visited with a descriptor equal to the one it was
last initialized from, leaving out the overrides both carry in `$env`, and its record of environment
reads holds at its new place, as the requirement on kept derived results checks it: it SHALL keep
its instance's state, and its rendered output and tokens SHALL be unchanged. When only the overrides
differ, the node SHALL take the new descriptor and the new environment, so that a later change is
judged against them. A node visited with any other descriptor of its component, or whose record of
environment reads does not hold at its new place, SHALL be initialized again from that descriptor's
fields, in its new environment, with the state the node holds, so that the output reflects the new
props, the environment and the kept state, and its tokens SHALL be those of the new render. A
descriptor that differs only in props the node's render did not read SHALL instead keep the node's
render, as the requirement on a node handed a new descriptor defines.

Two descriptors are equal when they hold the same values at the same places, with handlers
compared as follows:

- A handler the component's body can see, in a prop or in content at any depth, SHALL be resolved
  through the instance each descriptor was read from, and the two handlers SHALL be equal as the
  language's `==` finds them: the same handler node with equal captures.
- A handler the parent bound for one of the component's emits is never in the body's scope, so the
  two descriptors SHALL only both bind it or both leave it out.
- The text of a token SHALL NOT count. A parent that renders again numbers its handlers from one
  again, so the same text can name another handler, and other text can name an equal one.

When one old handler would be replaced by two different new handlers, the descriptors SHALL be
treated as different. Two old handlers replaced by one new handler are allowed, since a render that
hands one handler to two places writes it at both. A handler bound for an emit SHALL also answer
the same emit in both descriptors, as initializing the node checks.

So a node that renders again reaches only the nodes under it whose descriptors changed, and in
turn the nodes under those. A node beside the one that rendered, and a node above it that no
emitted action reached, SHALL NOT render again because of it.

#### Scenario: A parent that renders again re-initializes the child with the state it held
- **WHEN** a child node's state has been patched to `count` equal to `2`, its parent re-renders
  with a different value for one of the child's props that the child's body reads, and the host visits the child with the
  parent's new descriptor
- **THEN** the child's rendered output SHALL reflect the new descriptor's props and `count` equal
  to `2`

#### Scenario: A render reaches only the children whose descriptors changed
- **WHEN** a node renders `<Card title={first} onLogged=<State.Update last={action.text} /> />`
  and `<Card title="fixed" onLogged=<State.Update last={action.text} /> />`, it is dispatched
  against through the second card's emit so that `last` and `first` change, and a pass visits
  everything
- **THEN** the node and the first card SHALL hold instances of a new render, the card with the
  state it held
- **AND** the second card SHALL hold the state and the very rendered output it held before, and
  SHALL be settled once the pass ends
- **AND** a node beside the one dispatched against SHALL hold the instance and the rendered output
  it held before

#### Scenario: The same descriptor leaves the node alone
- **WHEN** a host visits a node twice with the same descriptor
- **THEN** the second visit SHALL return the first visit's rendered output with the same tokens

#### Scenario: An equal descriptor from a parent initialized again keeps the child
- **WHEN** a parent node is initialized again from other props, its output holds for a child a
  descriptor equal to the one the child was initialized from, and the host visits the child with it
- **THEN** the child SHALL NOT be initialized again and SHALL return the rendered output it held
- **AND** the handler the child holds SHALL be the one the parent's new instance holds at that
  place in the descriptor

#### Scenario: A visible handler with another capture is a changed descriptor
- **WHEN** `Page` renders `<Stack><Button onTapped=<State.Update taps={taps + 1} /> /></Stack>`,
  `Stack` is an authored component that renders its content, `taps` changes, and a pass visits the
  `Stack` node with `Page`'s new descriptor
- **THEN** the `Stack` node SHALL be initialized again with the state it held, because the handler
  in its content captures another value of `taps`

#### Scenario: A new override of a value the child didn't read keeps the child
- **WHEN** a parent renders `<Card title="x" />` and then, after a change to its state,
  `<Card title="x" env.locale="fr-FR" />`, and `Card`'s body reads no environment value
- **THEN** a pass that visits the `Card` node with the new descriptor SHALL keep it and its rendered
  output

#### Scenario: A new override of a value the child read renders it again
- **WHEN** a parent renders `<Card title="x" />` and then `<Card title="x" env.colorMode=dark />`,
  and `Card`'s body reads `env.colorMode`, whose value outside was `light`
- **THEN** the visit SHALL initialize the `Card` node again with the state it held, and its output
  SHALL reflect `dark`

### Requirement: A pass drops the nodes whose places left the output
The tree SHALL let a host mark the start and the end of a pass, and an instance SHALL live as long
as its place in the output. A node has rendered output no pass has walked from the moment it is
created, initialized again or rendered again by a dispatch until a pass that visits it ends. When a pass
ends:

- a node the pass visited SHALL be kept;
- a node under a visited node that held output no pass had walked SHALL be dropped unless the pass
  visited it: the host has just walked that output and did not find the node's place in it;
- a node under a visited node whose output a pass had already walked SHALL be kept whether or not
  the pass visited it: that output is what it was, so the place is still in it;
- a node under a node the pass did not reach SHALL be kept with it;
- a node under no node SHALL be dropped unless the pass visited it;
- every node under a dropped node SHALL be dropped with it.

A key visited again after its node was dropped SHALL get a new node in its initial state.

A dispatch MAY reach the tree while a pass is under way, and a visit SHALL count for what the node
held when the visit was made. A node that rendered again after the pass visited it, and that the
pass did not visit again, holds output the pass has not walked: the end of that pass SHALL NOT drop
a node under it for being unvisited and SHALL NOT take the node for walked, so it stays unsettled
for the next pass. A node under another that the pass visited before the other rendered again, and
not after, was read from output the other no longer holds: when the pass ends it SHALL be treated
as a node the pass did not visit.

The tree SHALL say of a node whether it is settled, which it is when neither it nor any node under
it holds output no pass has walked. A pass MAY leave everything under a settled node unvisited, so
a host that follows changes walks from the root only to the nodes that rendered and what is under
them. While a node does not render again the tree SHALL hand out the same rendered output for it,
so that a host can tell by identity that what it last read there still stands.

A host SHALL be able to remove a node, and the nodes under it, by key, for a place the output
still has and the host no longer wants an instance at.

#### Scenario: A use that left the output loses its state
- **WHEN** a `Page` renders a `Counter` only while its `shown` state is true, the `Counter` node's
  `count` is `5`, `shown` becomes false and a pass visits the `Page` node, and then `shown` becomes
  true again and a pass visits the `Page` node and the `Counter` descriptor at the same key
- **THEN** the first pass SHALL drop the `Counter` node
- **AND** the node of the later pass SHALL render with `count` equal to its initial value

#### Scenario: A visited node survives the pass
- **WHEN** a pass visits a node whose `count` is `5`
- **THEN** the node SHALL hold `count` equal to `5` after the pass ends

#### Scenario: A node whose parent left the output leaves with it
- **WHEN** a pass visits a child node and not the node under no node that it was found under
- **THEN** both SHALL be dropped when the pass ends

#### Scenario: An unchanged node keeps the nodes under it without a visit
- **WHEN** a node under a `Page` node holds `opened` equal to `5`, nothing has rendered since the
  last pass, and a pass visits the `Page` node and nothing under it
- **THEN** the node under it SHALL hold `opened` equal to `5` and the rendered output it held
- **AND** a later pass that visits the `Page` node after it has rendered again, and not the node
  under it, SHALL drop that node

#### Scenario: A settled node says so until something under it renders
- **WHEN** a pass has visited a `Top` node, a `Middle` node under it, a `Leaf` node under that, and
  a second node under `Top`
- **THEN** all four SHALL be settled
- **AND** after a dispatch against the `Leaf` node alone that renders it, the `Leaf`, `Middle` and
  `Top` nodes SHALL NOT be settled and the second node SHALL be
- **AND** after a pass that visits `Top`, `Middle` and `Leaf` and leaves the second node unvisited,
  all four SHALL be settled and the second node SHALL hold what it held

#### Scenario: A node dispatched against after its visit is not taken for walked
- **WHEN** a pass visits a settled `Page` node and nothing under it, the `Page` node is dispatched
  against so that it renders again before the pass ends, and the pass ends
- **THEN** the node under the `Page` node SHALL be kept with the state it held
- **AND** the `Page` node SHALL NOT be settled, and the pass that follows SHALL bring what is under
  it up to its new output

#### Scenario: A node visited before its parent rendered again does not count as visited
- **WHEN** a pass visits a `Page` node and the `Counter` node under it, the `Page` node is
  dispatched against so that it renders output that no longer holds a `Counter`, the pass visits the `Page`
  node again and nothing under it, and the pass ends
- **THEN** the `Counter` node SHALL be dropped and the `Page` node SHALL be settled
- **AND** a `Counter` visited at that key once the place returns SHALL be in its initial state
- **AND** a node whose place the new output still holds, visited before its parent rendered again
  and not after, SHALL likewise be dropped, never kept under a settled node with handlers of the
  instance its parent no longer holds

#### Scenario: A node is removed by key with the nodes under it
- **WHEN** a host removes a node that has a node under it, in a tree where nothing has rendered
  since the last pass
- **THEN** neither SHALL be in the tree, and a pass that visits only their parent SHALL NOT bring
  them back
- **AND** a visit at the removed key SHALL get a new node in its initial state

### Requirement: A handler runs against the instance whose body bound it
A host SHALL dispatch by naming the node whose rendered output it read, a token from that output,
and the action. The tree SHALL run the handler against the node whose body created it: the node
named, or the ancestor farthest from it that holds the same handler, which is where a handler a
component passed down through content or props was bound. The same handler is the very handler
that was handed down, not one equal to it: two instances of one component whose props and state
are equal hold handlers that are equal and are not one. The node dispatched against SHALL then
hold the state the dispatch produced, and the instance and the rendered output of the dispatch's
render when it rendered, or the ones it held when the dispatch kept its render. A token the named node does not
hold SHALL fail with `nx-ir-handler-token`.

A dispatch that renders replaces the instance of every node it renders, and the nodes under such a
node hold handlers of the instance it had until a pass visits them again. A host therefore runs a pass
after every dispatch. A dispatch that reaches the tree before that pass, for a handler that
was handed down from an instance since replaced, SHALL fail with `nx-ir-handler-token` and SHALL
change nothing: it SHALL NOT run the handler against a node that was only handed it.

#### Scenario: A tap patches the instance that bound the handler
- **WHEN** a host dispatches the token of `onTapped=<State.Update count={count + 1} />` read from a
  `Counter` node's output, twice, each time with the token of the output then current
- **THEN** the node's state SHALL hold `count` equal to `2`

#### Scenario: A handler in a content child patches its owner
- **WHEN** `Page` renders `<Stack><Button onTapped=<State.Update taps={taps + 1} /> /></Stack>`, `Stack`
  is an authored component that renders its content, and the host dispatches the button's token
  through the `Stack` node whose output held it
- **THEN** the `Page` node's state SHALL hold `taps` equal to `1`
- **AND** the `Stack` node's own state SHALL be unchanged

#### Scenario: A retired token is refused
- **WHEN** a host dispatches a token from an output a previous dispatch replaced by rendering
- **THEN** the dispatch SHALL fail with `nx-ir-handler-token`

#### Scenario: A component nested under itself is told from its ancestor
- **WHEN** a component renders another instance of itself, both instances hold equal props and
  state, and the host dispatches the token of the inner instance's own handler through the inner
  node
- **THEN** the inner node's state SHALL be patched
- **AND** the outer node's state SHALL be unchanged

#### Scenario: An event from output a dispatch replaced is refused
- **WHEN** `Page` puts a button of its own, whose label reads the state field its handler patches,
  in the content of a `Box`, the host dispatches the button's token through the `Box` node, and dispatches the same token again with no pass
  between
- **THEN** the second dispatch SHALL fail with `nx-ir-handler-token`
- **AND** the `Page` node SHALL hold what the first dispatch left and the `Box` node what it held
- **AND** after a pass, the token the `Box` node's output then holds SHALL dispatch against `Page`

### Requirement: An emitted action is carried to the handler the parent bound
An effect a dispatch returns SHALL be routed when its type is an action the node's component emits
and the node holds a handler its parent bound for that emit: the tree SHALL dispatch the effect as
the action of that handler, against the node whose body created it. Any other effect SHALL be
returned to the host with the name of the component that produced it. A node that holds a handler
its parent bound for the emit, where no node above it holds that handler any longer because a
dispatch replaced the parent's instance and no pass has followed, SHALL fail the dispatch
with `nx-ir-handler-token`: the action SHALL NOT be returned to the host as though nobody had bound
a handler for it. Routing SHALL continue until
no effect remains to route. Within one host dispatch a node SHALL be dispatched against at most
once, with every action routed to it in one batch in the order the actions were produced, so that
no entry names a token an earlier dispatch of the same chain retired.

#### Scenario: A child's emit patches the parent
- **WHEN** `Card` emits `Logged`, `Page` renders `<Card onLogged=<State.Update last={action.text} /> />`,
  and a handler in `Card` returns `<Card.Logged text="hi" />`
- **THEN** the `Page` node's state SHALL hold `last` equal to `"hi"`
- **AND** the host SHALL receive no effect

#### Scenario: Two emitted actions reach the parent in one batch
- **WHEN** one handler in a child returns two actions its parent bound handlers for, each patching
  a different state field of the parent that the parent's body reads
- **THEN** the parent node's state SHALL hold both patches
- **AND** the parent SHALL have rendered once

#### Scenario: An emit whose handler belongs to a replaced render is refused
- **WHEN** a child's handler emits an action its parent bound a handler for, which patches a state
  field the parent's body reads, the host dispatches it, and dispatches the child's handler again with no pass between; or the parent's own
  handler is dispatched first and the child's after it
- **THEN** the later dispatch SHALL fail with `nx-ir-handler-token`
- **AND** every node SHALL hold what it held before that dispatch
- **AND** after a pass the child's handler SHALL dispatch and its emit SHALL reach the parent

#### Scenario: An emit nobody bound is a host effect
- **WHEN** a handler returns an action its component emits and no parent bound a handler for it
- **THEN** the dispatch SHALL return that action as an effect naming the component that produced it

### Requirement: A change to the tree is all or nothing
A host dispatch that fails SHALL leave every node holding the descriptor, the instance and the
rendered output it held before the dispatch, whether it fails in the handler it named or in one an
effect was routed to, and SHALL report the runtime's diagnostic for the failure. The tree SHALL also let a
host run a pass as one change: when the pass fails, the nodes, what they hold and the
record of what the pass visited SHALL be as they were before it. Whenever a change is undone, the
tree's kept derived results, its revision counter, its indexes of readers and users, and its change
set SHALL be as they were before the change too.

#### Scenario: A chain that fails part way restores every node
- **WHEN** a child's handler patches the child and emits an action whose parent handler fails
- **THEN** the dispatch SHALL fail with the parent handler's diagnostic
- **AND** the child node's state SHALL be what it was before the dispatch
- **AND** a token of the child's earlier output SHALL still dispatch

#### Scenario: A pass that fails leaves the tree as it was
- **WHEN** a pass run as one change re-initializes one node and then fails at another
- **THEN** the first node SHALL hold the descriptor, instance and rendered output it held before
  the pass

### Requirement: A change to a root value reaches only the nodes that read it
Each node SHALL record the direct inputs of its last render, as the reactivity capability defines:
its root reads, the kept results it used, and its override-supplied reads with the values found, a
read made while evaluating a default, an entry-module provide, or a formula or bundle entry carried
in `$env` or in an `envProvide`, for another read, reaching the node through the kept result or the
override-supplied read it was made in. The tree SHALL keep an index from each environment value to
the nodes and kept results that read its root value, directly or through the lookup of an
override-supplied read, and SHALL keep it current as nodes render, are replaced, removed and
dropped. `set_environment` SHALL replace the tree's root values and SHALL find the nodes to check
through the index alone, from the values whose root value changed, without walking the tree. For
each of those nodes, the tree SHALL check the node's record at its environment as the requirement on
kept derived results defines, and SHALL render again, with the state it holds, each node for which a
direct input changed. It SHALL leave the others alone. A node rendered this way SHALL hold rendered
output no pass has walked, as after a dispatch, so that the host's next pass walks it and what is
under it. `set_environment` SHALL be all or nothing: when a root value is refused or a render fails,
the tree SHALL be as it was.

#### Scenario: Only readers render again
- **WHEN** a tree holds `Page`, a `Header` node and a `Body` node under it, only `Header`'s body
  reads `env.colorMode`, and a host calls `set_environment` with `colorMode` changed from `light` to
  `dark`
- **THEN** the `Header` node SHALL render again with the state it held and read `dark`
- **AND** `Page` and `Body` SHALL hold the rendered output they held, and `Body` SHALL be settled

#### Scenario: A reader under an override that hides the change is left alone
- **WHEN** a `Card` node reads `env.colorMode` under `<div env.colorMode=light>`, and a host changes
  the root value of `colorMode` to `dark`
- **THEN** the `Card` node SHALL NOT render again

#### Scenario: A reader through a default renders again
- **WHEN** a `Footer` node reads `env.color`, whose default reads `env.colorMode`, and a host
  changes the root value of `colorMode`
- **THEN** the `Footer` node SHALL render again and read the colors of the new mode

#### Scenario: A refused root value changes nothing
- **WHEN** a host calls `set_environment` with a value of the wrong type for one value and a valid
  change to another
- **THEN** the call SHALL fail and every node SHALL hold what it held

#### Scenario: A reader through an override formula renders again
- **WHEN** a `Cell` node reads `env.gap` under `<Row env.gap={env.space * 2}>`, and a host changes
  the root value of `space` from `4` to `6`
- **THEN** the `Cell` node SHALL render again and read `12`
