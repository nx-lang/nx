# Scenario coverage

Every scenario of `specs/agent-host-package/spec.md`, with the tests that cover it, for the pull
request description (tasks 10.3 and 11.8). The tests are in `packages/agent/test/`. Each entry was
checked by a script: the scenario is in the spec, and the test file holds a test of that title. A
title ending in `...` is the fixed part of one written for several cases.

116 scenarios, and every one is covered.

Three need a note:

- *A Worker imports the run-time half*: the test reads the import specifiers of the built files
  each entry point reaches. Nothing was bundled for a Worker.
- *A host that does not use the AI SDK installs the package*: the test holds the manifest and the
  imports to it. It was also done by hand on 2026-10-04: the packed tarball installed with no `ai`,
  with no peer warning, `.`, `./normalize` and `./execute` imported, and `./ai-sdk` alone failed
  with `ERR_MODULE_NOT_FOUND` for `ai`.
- The scenario this change adds to `package-release-automation`, *The agent package is published
  after the runtime it depends on*, has no test: `scripts/publish-packages.mjs --dry-run` over the
  eight packed tarballs lists `@nx-lang/ir-runtime` and then `@nx-lang/agent` (task 8.2).

## The agent host package is published with separate compile-time and run-time entry points

- **A Worker imports the run-time half**: `entry-points.test.ts`: loads nothing from outside the package but...
- **A host that does not use the AI SDK installs the package**: `entry-points.test.ts`: only the AI SDK entry point imports ai, which is an optional peer
- **The library's types are importable from the package**: `consumer-types.test.ts`: the package's types resolve for a consumer under moduleResolution...; `generated-types.test.ts`: the published library types are what nxlang typegen @nx/agent generates
- **The compile-time half takes schemas as data**: `entry-points.test.ts`: loads nothing from outside the package but...; `normalize.test.ts`: a program artifact is a schema source as it stands

## The public API has fixed names

- **A host passes its artifact and context type**: `normalize.test.ts`: the query lists ToolContext as host-supplied, by module and name; `normalize-tools.test.ts`: a context parameter is left out of the schema and recorded with its declared type
- **An absent optional field is omitted**: `normalize.test.ts`: an optional field with no value is absent, and an agent with no tools has an empty list; `normalize-tools.test.ts`: a result with no JSON form is a warning, and the tool has no output schema
- **Creation errors carry their codes**: `execute.test.ts`: an unknown format version is refused, naming the version given and the versions supported; `execute.test.ts`: a function missing from the program is refused at creation, naming the tool, the module and the function; `execute.test.ts`: a kind with no executor is refused at creation, naming the tool and the kind; `execute.test.ts`: an http tool with no request function is refused at creation

## An evaluated Agent normalizes to a storable definition

- **An agent with a function tool normalizes**: `normalize.test.ts`: an evaluated agent normalizes to a definition
- **The definition survives storage**: `normalize-tools.test.ts`: the definition survives storage: it is plain JSON, and parsing it back gives an equal definition; `end-to-end.test.ts`: an agent compiled in one place runs in another, from the stored definition and the emitted IR alone
- **An invalid agent yields no definition**: `normalize-tools.test.ts`: every problem of one agent is reported in one pass
- **A value that is not an Agent is refused**: `normalize.test.ts`: a value that is not an Agent is refused, naming what it is
- **Duplicate document titles are rejected**: `normalize.test.ts`: duplicate document titles are rejected, naming the title

## A normalized tool follows the MCP tool shape

- **A definition has the MCP fields**: `normalize-tools.test.ts`: a function tool takes its name, description and schemas from its function
- **An array result is not wrapped**: `normalize-tools.test.ts`: an array result is not wrapped; `execute.test.ts`: a function tool returns the function's canonical result, unwrapped

## Model-facing tool names are snake_case and unique

- **A camelCase function name is converted**: `names.test.ts`: a function name converts to snake_case; `normalize-tools.test.ts`: a function tool takes its name, description and schemas from its function
- **Acronyms and element-style names are converted**: `normalize-tools.test.ts`: acronyms and element-style names are converted
- **An authored name is kept**: `normalize-tools.test.ts`: an authored name and description win over the function's
- **An authored name that is not snake_case is rejected**: `normalize-tools.test.ts`: an authored name that is not snake_case is rejected, not converted
- **Duplicate names are rejected**: `normalize-tools.test.ts`: duplicate names are rejected with one diagnostic naming the name and both positions

## Tool descriptions come from the author or the function's doc comment

- **The doc comment is the default description**: `normalize-tools.test.ts`: a function tool takes its name, description and schemas from its function
- **An authored description wins**: `normalize-tools.test.ts`: an authored name and description win over the function's
- **A tool with no description is rejected**: `normalize-tools.test.ts`: a tool with no description is rejected, naming the tool and pointing at its function
- **Parameter documentation reaches the schema**: `normalize-tools.test.ts`: a function tool takes its name, description and schemas from its function

## Tool schemas come from the declaration schema export

- **Required and optional parameters**: `normalize-tools.test.ts`: a function tool takes its name, description and schemas from its function; `normalize-tools.test.ts`: an array result is not wrapped
- **A parameter with a default is not required**: `normalize-tools.test.ts`: a parameter with a default, an optional parameter and no parameters
- **A function-typed parameter is rejected**: `normalize-tools.test.ts`: a function-typed parameter is rejected, keeping the export's message
- **A function the program does not declare is rejected**: `normalize-tools.test.ts`: a function the program does not declare is rejected, naming the module and the function
- **A schema keyword the package does not know is kept**: `normalize-tools.test.ts`: a schema keyword the package does not know is kept; `ai-sdk.test.ts`: the SDK is given the input schema as it is stored, keywords the package does not know included

## ToolContext parameters are filled by the host and hidden from the model

- **A context parameter is left out of the schema**: `normalize-tools.test.ts`: a context parameter is left out of the schema and recorded with its declared type
- **A context parameter declared through an alias is a context parameter**: `normalize-tools.test.ts`: a context parameter declared through an alias is a context parameter; `execute.test.ts`: a base-typed parameter receives the host's subtype
- **A function with only a context parameter takes no input**: `normalize-tools.test.ts`: a context parameter is left out of the schema and recorded with its declared type
- **A context parameter the host cannot fill is rejected**: `normalize-tools.test.ts`: a context parameter the host cannot fill is rejected: no context type was named
- **A sequence of contexts is rejected**: `normalize-tools.test.ts`: a sequence of contexts is rejected
- **A context held in another record is rejected**: `normalize-tools.test.ts`: a context held in another record is rejected, at any depth
- **A context of a subtype the host did not name is still found**: `normalize-tools.test.ts`: a context of a subtype the host did not name is still found

## Annotations are fixed per tool and never authored

- **A function tool is read-only**: `annotations.test.ts`: a function tool is read-only, idempotent and closed-world; `normalize-tools.test.ts`: a function tool takes its name, description and schemas from its function
- **A POST tool is a non-idempotent write**: `annotations.test.ts`: an HTTP tool's hints follow its method; `normalize-tools.test.ts`: each method has its own annotations, and the authored call limit is kept
- **A GET tool is read-only**: `normalize-tools.test.ts`: an HTTP tool normalizes to a fixed operation on its connection

## WebSearchTool is described as a provider tool and never executed

- **A web search tool carries its domains**: `normalize-tools.test.ts`: a web search tool is described as a provider tool with its domains
- **A provider tool has no execute**: `execute.test.ts`: a provider tool has no execute

## An HttpTool normalizes to a fixed operation on its connection

- **An HTTP tool is normalized**: `normalize-tools.test.ts`: an HTTP tool normalizes to a fixed operation on its connection
- **A non-HTTPS base URL is rejected**: `normalize-tools.test.ts`: a base URL that is not https is rejected, naming the connection and the scheme
- **A base URL with a query is rejected**: `normalize-tools.test.ts`: a base URL with a query, a fragment, user information or no scheme is rejected, naming the connection
- **A base URL a parser would rewrite is rejected**: `normalize-tools.test.ts`: a base URL with a query, a fragment, user information or no scheme is rejected, naming the connection; `http.test.ts`: a base URL a parser would rewrite is refused, naming the spelling to write
- **A path with a dot segment is rejected**: `normalize-tools.test.ts`: a malformed path is rejected, naming the tool and the path
- **An arguments function with the wrong result type is rejected by the compiler**: `normalize-tools.test.ts`: an arguments function with the wrong result type is rejected by the compiler

## Normalization diagnostics are structured

- **Two problems are reported together**: `normalize-tools.test.ts`: two problems are reported together, each with its own code and path
- **A warning does not block the definition**: `normalize-tools.test.ts`: a warning does not block the definition

## A host describes its own Tool subtypes

- **A host tool type is described**: `normalize-tools.test.ts`: a host tool type is described by its describer
- **One element resolves to two tools**: `normalize-tools.test.ts`: one element resolves to two tools, in the order returned, at its position
- **An unregistered tool type is rejected**: `normalize-tools.test.ts`: an unregistered tool type is rejected, naming the type
- **A described tool collides with a derived name**: `normalize-tools.test.ts`: a name is unique across derived, authored and described tools

## Executable tools are created from a stored definition and a linked program

- **Tools are created without the compiler**: `end-to-end.test.ts`: an agent compiled in one place runs in another, from the stored definition and the emitted IR alone; `entry-points.test.ts`: loads nothing from outside the package but...
- **An unknown format version is refused**: `execute.test.ts`: an unknown format version is refused, naming the version given and the versions supported
- **A damaged definition is reported, not thrown**: `execute.test.ts`: a definition that is not as the package wrote it is reported with its path, and never as a TypeError; `execute-http.test.ts`: a tool that is not as the package wrote it fails the evaluation, and is never a TypeError
- **A function missing from the program is refused at creation**: `execute.test.ts`: a function missing from the program is refused at creation, naming the tool, the module and the function
- **A host checks a definition when it compiles**: `execute.test.ts`: a host checks a definition against the program it linked, with no executors and no request function
- **A kind with no executor is refused at creation**: `execute.test.ts`: a kind with no executor is refused at creation, naming the tool and the kind
- **An aborted call does nothing**: `execute.test.ts`: an aborted call does nothing

## A function tool runs its function under the evaluation budget

- **An optional context field set to undefined is absent**: `execute.test.ts`: a base-typed parameter receives the host's subtype; `json.test.ts`: a member that is undefined is left out at any depth, and everything else is kept as it is
- **A context record that holds itself is refused, not followed**: `execute.test.ts`: a base-typed parameter receives the host's subtype; `json.test.ts`: a value that holds itself, one nested very deeply and a very large one all end, without the engine's stack
- **A function tool returns the function's result**: `execute.test.ts`: a function tool returns the function's canonical result, unwrapped
- **A result carries what the call used**: `execute.test.ts`: a result carries what the call used
- **A failure after the call began carries what it used**: `execute.test.ts`: the package's default budget applies when the host sets none
- **Calls started together carry their own numbers**: `execute.test.ts`: calls started together carry their own numbers
- **A failure before the call carries no usage**: `execute.test.ts`: a failure before the call carries no usage
- **A failure the runtime finds carries usage**: `execute.test.ts`: input of the wrong type is invalid input, with the runtime's diagnostic and usage
- **The host's own usage object is left alone**: `execute.test.ts`: the host's own usage object is left alone
- **An HTTP failure after the arguments function ran carries usage**: `execute-http.test.ts`: a structural failure never reaches the network; `execute-http.test.ts`: an error the request function throws fails the call as request-failed, with the usage of the arguments function's call
- **Input of the wrong type is invalid input**: `execute.test.ts`: input of the wrong type is invalid input, with the runtime's diagnostic and usage
- **A missing required argument is invalid input**: `execute.test.ts`: a missing required argument is invalid input, naming it
- **A boundary failure of a kind added later is invalid input**: `classify.test.ts`: a boundary failure of a kind added later is invalid input; `classify.test.ts`: one failure that is not of the boundary makes it an evaluation failure
- **A runaway function hits the budget**: `execute.test.ts`: the package's default budget applies when the host sets none
- **Input that is too large is invalid input**: `execute.test.ts`: input that is too large is invalid input, and the function is not called
- **The host's context does not count against the model's cap**: `execute.test.ts`: the host's context does not count against the model's cap
- **A context that is too large is a resource limit**: `execute.test.ts`: a context that is too large is a resource limit, and the function is not called
- **A function with two context parameters is given the context twice**: `execute.test.ts`: a function with two context parameters is given the context twice
- **The host's own input limit is not used**: `execute.test.ts`: the host's own input limit is not used
- **A host raises the limits through the package's options**: `execute.test.ts`: a host raises and lowers the limits through the package's options
- **The arguments function runs under the same limits**: `execute-http.test.ts`: the arguments function runs under the same two input limits
- **The package's default applies when the host sets no budget**: `execute.test.ts`: the package's default budget applies when the host sets none; `execute-http.test.ts`: the arguments function fails with a function tool's codes, without a request
- **The host's budget is used**: `execute.test.ts`: the host's budget is used when it sets one; `execute-http.test.ts`: the arguments function fails with a function tool's codes, without a request
- **A failure inside the function is an evaluation failure**: `execute.test.ts`: a failure inside the function is an evaluation failure

## Context parameters are filled from the host's context record at each call

- **The host supplies identity**: `execute.test.ts`: the host supplies identity: the record is passed with callId set
- **A base-typed parameter receives the host's subtype**: `execute.test.ts`: a base-typed parameter receives the host's subtype
- **The model cannot supply the context**: `execute.test.ts`: the model cannot supply the context
- **A missing context record is reported**: `execute.test.ts`: a missing context record is reported, naming the parameter, without calling the function

## An HTTP request is built structurally from the tool and its arguments

- **A path parameter is substituted and encoded**: `http.test.ts`: a path parameter is substituted and encoded
- **A base path is kept**: `http.test.ts`: a base path is kept, joined by exactly one slash
- **Query parameters are encoded in order**: `http.test.ts`: query parameters are encoded in order
- **A traversal value is refused**: `http.test.ts`: a traversal value is refused
- **A missing path parameter is refused**: `http.test.ts`: a missing path parameter is refused, naming the placeholder
- **An undeclared path parameter is refused**: `http.test.ts`: an undeclared path parameter is refused, naming it
- **A body is sent as plain JSON**: `http.test.ts`: the spec's body example; `http.test.ts`: a body is sent as plain JSON, without $type at any depth
- **A body on GET or DELETE is refused**: `http.test.ts`: put, post and patch carry a body; get and delete refuse one
- **A host rebuilds the request from stored configuration**: `http.test.ts`: a host rebuilds the same request from stored configuration and structured arguments

## An HTTP tool evaluates its arguments and calls the host's request function

- **The host's function makes the call**: `execute-http.test.ts`: the host's function makes the call, and what it answers is the output
- **A host forwards the arguments instead of the request**: `execute-http.test.ts`: a host forwards the arguments instead of the request, and the other process builds the same one
- **A non-2xx status is an output, not a failure**: `execute-http.test.ts`: a non-2xx status is an output, not a failure
- **Extra result properties are kept**: `execute-http.test.ts`: extra result properties are kept
- **A structural failure never reaches the network**: `execute-http.test.ts`: a structural failure never reaches the network
- **A function that does not return HttpArguments is refused at the call**: `execute-http.test.ts`: a function that does not return HttpArguments is refused at the call, naming the function
- **Arguments are evaluated without sending**: `execute-http.test.ts`: arguments are evaluated without sending: the structured arguments and the request
- **A host policy error keeps its code**: `execute-http.test.ts`: a host policy error keeps its code
- **The package sets no idempotency header**: `execute-http.test.ts`: the package sets no idempotency header, and hands over the callId so the host can

## A host executes its own tool kinds

- **A host kind is executed**: `execute.test.ts`: a host kind is executed with the stored config, the input and the call context
- **A host executor failure is a result**: `execute.test.ts`: a host executor failure is a result, with the code of the package's tool error when it threw one

## Executable tools adapt to AI SDK tools

- **A host using the adapter reads what a call used**: `ai-sdk.test.ts`: a host using the adapter reads what a call used, on a success and on a budget failure
- **The adapter waits for the result function**: `ai-sdk.test.ts`: the adapter waits for the result function before it returns the output; `ai-sdk.test.ts`: what the result function throws, or rejects with, is the tool's error
- **The result function sees a failure too**: `ai-sdk.test.ts`: a host using the adapter reads what a call used, on a success and on a budget failure
- **A function tool is given to streamText**: `ai-sdk.test.ts`: a function tool is given to streamText, and the model receives the function's canonical result
- **A host-derived call identifier is used**: `ai-sdk.test.ts`: the callId is the SDK's toolCallId unless the host derives one
- **A failure becomes a tool error**: `ai-sdk.test.ts`: a failure is thrown as the package's tool error, with the result's code; `ai-sdk.test.ts`: through generateText, a failing tool reaches the model as a tool error
- **The model is told why a call failed only when it can correct it**: `ai-sdk.test.ts`: through generateText, what a request function threw does not reach the model, for a read and for a write; `ai-sdk.test.ts`: the model is told why a call failed only when it can correct it; the host is told every time
- **A provider tool is mapped by the host**: `ai-sdk.test.ts`: a provider tool is mapped by the host, which is given its definition
- **An unmapped provider tool is an error**: `ai-sdk.test.ts`: an unmapped provider tool is an error naming the tool, not a silent omission

# The declaration schema export

The scenarios this change adds to, or touches in, the two requirements it modifies in
`declaration-schema-export`, with the tests in `crates/nx-api/src/schema_tests.rs`. The answer is
also checked through both bindings, in `bindings/wasm/test/schema.test.ts` and
`bindings/node/test/sdk-node.test.ts`, and `bindings/wasm/test/parity.test.ts` compares the two.

- **A parameter declared through an alias names what the alias denotes**: `an_alias_of_a_listed_type_is_host_supplied_as_the_type_it_denotes`; `an_alias_of_an_occurrence_of_a_listed_type_is_not_host_supplied`
- **A context parameter is left out of the input schema**: `a_context_parameter_is_left_out_of_the_input_schema`
- **A subtype of a listed type is host-supplied too**: `a_subtype_of_a_listed_type_across_a_library_boundary_is_host_supplied`
- **A result holding an object field is open at that field**: `a_result_holding_an_object_field_is_open_at_that_field`
- **Without the list every parameter is an argument**: `without_the_list_every_parameter_is_an_argument`
- **An alias of a listed type is host-supplied too**: `an_alias_of_a_listed_type_is_host_supplied_as_the_type_it_denotes`; `an_alias_of_an_occurrence_of_a_listed_type_is_not_host_supplied`
- **A listed type under an occurrence is named, and the parameter is kept**: `a_listed_type_under_an_occurrence_is_named_and_the_parameter_is_kept`; `the_abstract_listed_type_and_a_subtype_the_host_does_not_use_are_named`
- **A listed type held as a field is named, at any depth**: `a_listed_type_held_as_a_field_is_named_at_any_depth`; `two_parameters_of_one_record_are_both_named`; `a_listed_type_in_a_union_case_through_an_alias_and_as_a_type_argument_is_named`
- **A record that is two listed types is named for each**: `a_record_that_is_two_listed_types_is_named_for_each`
- **A type derived from a listed type holds none of it**: `a_derived_type_of_a_listed_type_holds_none_of_it`
- **A parameter that holds no listed type says nothing**: `a_parameter_that_holds_no_listed_type_says_nothing`; `a_host_supplied_parameter_names_nothing_it_holds`; `with_no_listed_types_nothing_is_named`
- **A record that holds itself ends**: `a_record_that_holds_itself_ends_and_is_named_once`; `a_record_that_applies_itself_to_a_growing_argument_ends`

The other 8 scenarios of the two requirements are carried in the delta unchanged, as a modified
requirement is written whole, and are covered by the tests the export landed with.
