# NX Grammar Spec (Token-Oriented, AI-Friendly)

This is the machine-oriented grammar for NX. It is:
- Non-left-recursive and left-factored (single-token lookahead chooses branches).
- Operator-free for conventional expressions; a separate precedence table drives a Pratt parser.
- Token-oriented: productions expand to token kinds (UPPER_SNAKE) or other nonterminals (CamelCase).
- AST-oriented: each rule includes its node type and fields.
- Naming follows Roslyn conventions: AST node types use the `Syntax` suffix (e.g., `FunctionDefinitionSyntax`, `ValueIfExpressionSyntax`).

Source of truth for language shape: nx-grammar.md. This spec re-expresses it in a parser-ready form.

## Token Vocabulary

Terminals are UPPER_SNAKE token kinds produced by the lexer. Lexeme hints are illustrative; actual lexing rules are defined by the lexer.

Keywords
- IMPORT ("import")
- TYPE ("type")
- ENUM ("enum")
- LET ("let")
- IF ("if"), ELSE ("else"), IS ("is")
- FOR ("for"), IN ("in")
- RAW ("raw")

Primitive types (keywords)
- STRING ("string")
- INT ("int"), LONG ("long")
- FLOAT ("float"), DOUBLE ("double")
- BOOLEAN ("boolean")
- VOID ("void")
- OBJECT ("object")

Identifiers and names
- IDENTIFIER (letters, digits, underscore; starts with letter/underscore)
- MARKUP_IDENTIFIER (letters, digits, underscore, hyphen)

Literals
- STRING_LITERAL
- INT_LITERAL
- REAL_LITERAL
- HEX_LITERAL
- BOOL_LITERAL (true|false)
- NULL_LITERAL (null)

 Punctuation and operators
- LT (<), GT (>)
- LPAREN (() , RPAREN ())
- LBRACE ({), RBRACE (})
- LBRACK ([), RBRACK (])
- SLASH (/), COLON (:), COMMA (,), DOT (.)
- EQ (=)
- QMARK (?)
- PIPE (|)
 - ELLIPSIS (...)
 - PLUS (+), MINUS (-), STAR (*), SLASH (/)
 - LT_EQ (<=), GT_EQ (>=), EQ_EQ (==), BANG_EQ (!=)
 - AMP_AMP (&&), PIPE_PIPE (||)

Text content tokens (inside text elements)
- TEXT_CHUNK (sequence of text chars excluding '<', '&', '{'; backslashes are literal unless part of an escaped brace)
- ENTITY (named or numeric entity; e.g., &amp; &#10;)
- ESCAPED_LBRACE (`"\{"`)
- ESCAPED_RBRACE (`"\}"`)
- Only `"\{"` and `"\}"` sequences are treated as escapes; any other backslash-prefixed sequences remain literal text.

Special
- EOF (end of file)

Trivia (whitespace/comments; skipped by lexer)
- WHITESPACE (spaces, tabs, newlines)
- LINE_COMMENT ("//" to end of line)
- BLOCK_COMMENT ("/*" … "*/"; nests with same-kind openers)
- HTML_BLOCK_COMMENT ("<!--" … "-->"; nests with same-kind openers)

Notes
- The lexer does not produce tokens for trivia; they are attached as trivia or discarded.
- Comments and whitespace may appear between any tokens.
- Block comments are nestable with same-kind openers only. The lexer maintains a depth counter: increment on opener, decrement on closer, emit one token at depth 0. Unterminated blocks are lexing errors.
- Comments are not recognized inside string literals or text content tokens (TEXT_CHUNK/ENTITY/ESCAPED_*).

Raw embed tokens
- RAW_TEXT_CHUNK — produced only inside raw embed content; scanners treat '{', '}', '&' as ordinary characters.

## Operator Precedence (Pratt)

Conventional expressions (non-markup) use a Pratt parser with the following precedence and associativity. Higher number binds tighter.

 140: Paren function call, member access
- Paren function call: led token: LPAREN … RPAREN, left-associative
  - form: callee LPAREN [Expr (COMMA Expr)*] RPAREN → ParenFunctionCall(callee, args)
- Member access: led token: DOT IDENTIFIER, left-associative
  - form: left DOT IDENTIFIER → MemberAccess(left, name)
  - Note: Handles both property/field access on values and enum member access; semantic analysis distinguishes based on whether left resolves to a type or value

 130: Prefix unary, right-associative
 - Prefix minus: nud token: MINUS
   - form: MINUS Expr → PrefixUnaryExpression(op: MINUS, expr)

 120: Multiplicative, left-associative
- STAR (*), SLASH (/)

110: Additive, left-associative
- PLUS (+), MINUS (-)

90: Relational, left-associative
- LT (<), GT (>), LT_EQ (<=), GT_EQ (>=)

80: Equality, left-associative
- EQ_EQ (==), BANG_EQ (!=)

40: Logical AND, left-associative
- AMP_AMP (&&)

30: Logical OR, left-associative
- PIPE_PIPE (||)

20: Conditional, right-associative
- led token: QMARK … COLON …
  - form: condition QMARK consequent COLON alternative → Conditional(condition, consequent, alternative)

Grouping
- LPAREN Expr RPAREN binds as a primary (handled in nud for LPAREN).

 Notes
 - Unit literal is distinct: LPAREN RPAREN → Unit.

## Grammar (Left-Factored, Token-Oriented)

Nonterminals are CamelCase. Terminals are UPPER_SNAKE tokens.

ModuleDefinition (AST: ModuleDefinitionSyntax)
- ModuleDefinition → ImportStatement* ModuleMember* Element? EOF
  - fields: imports: ImportStatementSyntax[], members: ModuleMemberSyntax[], moduleElement?: MarkupElementSyntax

ModuleMember (AST: ModuleMemberSyntax is a sum type)
- ModuleMember → TypeDefinition
- ModuleMember → ValueDefinition
- ModuleMember → FunctionDefinition

ImportStatement (AST: ImportStatementSyntax)
- ImportStatement → IMPORT QualifiedName
  - fields: name: QualifiedNameSyntax

TypeDefinition (AST: TypeDefinitionSyntax is a sum type)
- TypeDefinition → TypeAliasDefinition (TypeAliasDefinitionSyntax)
- TypeDefinition → EnumDefinition (EnumDefinitionSyntax)

TypeAliasDefinition (AST: TypeAliasDefinitionSyntax)
- TypeAliasDefinition → TYPE IDENTIFIER EQ Type
  - fields: name: string, type: TypeSyntax

EnumDefinition (AST: EnumDefinitionSyntax)
- EnumDefinition → ENUM IDENTIFIER EQ EnumMemberList
  - fields: name: string, members: EnumMemberSyntax[]

EnumMemberList
- EnumMemberList → EnumMemberListLead EnumMemberListTail

EnumMemberListLead
- EnumMemberListLead → EnumMember
- EnumMemberListLead → PIPE EnumMember

EnumMemberListTail
- EnumMemberListTail → PIPE EnumMember EnumMemberListTail
- EnumMemberListTail → ε

EnumMember (AST: EnumMemberSyntax)
- EnumMember → IDENTIFIER
  - fields: name: string

ValueDefinition (AST: ValueDefinitionSyntax)
- ValueDefinition → LET IDENTIFIER ValueDefinitionTypeOpt EQ RhsExpression
  - fields: name: string, type?: TypeSyntax, value: ExpressionSyntax

ValueDefinitionTypeOpt
- ValueDefinitionTypeOpt → COLON Type
- ValueDefinitionTypeOpt → ε

Type (AST: TypeSyntax)
- Type → PrimitiveType TypeOptModifier
- Type → UserDefinedType TypeOptModifier
  - fields: kind: "primitive"|"user", name: string (qualified), modifier?: "nullable"|"sequence"

TypeOptModifier
- TypeOptModifier → QMARK
- TypeOptModifier → LBRACK RBRACK
- TypeOptModifier → ε

PrimitiveType (AST: PrimitiveTypeSyntax)
- PrimitiveType → STRING | INT | LONG | FLOAT | DOUBLE | BOOLEAN | VOID | OBJECT
  - fields: name: "string"|"int"|"long"|"float"|"double"|"boolean"|"void"|"object"

UserDefinedType (AST: UserTypeSyntax)
- UserDefinedType → QualifiedName
  - fields: name: QualifiedNameSyntax

FunctionDefinition (AST: FunctionDefinitionSyntax is a sum type)
- FunctionDefinition → ElementFunctionDefinition        (ElementFunctionDefinitionSyntax)
- FunctionDefinition → ParenFunctionDefinition          (ParenFunctionDefinitionSyntax)

ElementFunctionDefinition (AST: ElementFunctionDefinitionSyntax)
- ElementFunctionDefinition → LET LT ElementName PropertyDefinition* SLASH GT FunctionReturnTypeOpt EQ RhsExpression
  - fields: elementName: QualifiedMarkupNameSyntax, parameters: PropertyDefinitionSyntax[], returnType?: TypeSyntax, body: ExpressionSyntax

ParenFunctionDefinition (AST: ParenFunctionDefinitionSyntax)
- ParenFunctionDefinition → LET IDENTIFIER LPAREN ParenParameterListOpt RPAREN FunctionReturnTypeOpt EQ RhsExpression
  - fields: name: string, parameters: PropertyDefinitionSyntax[], returnType?: TypeSyntax, body: ExpressionSyntax

ParenParameterListOpt
- ParenParameterListOpt → ParenParameterList
- ParenParameterListOpt → ε

ParenParameterList
- ParenParameterList → PropertyDefinition (COMMA PropertyDefinition)*
  - Note: we intentionally reuse `PropertyDefinition` here so paren-style and element-style
    declarations share the same syntax tree representation for parameters (name, type, default).

FunctionReturnTypeOpt
- FunctionReturnTypeOpt → COLON Type
- FunctionReturnTypeOpt → ε

PropertyDefinition (AST: PropertyDefinitionSyntax)
- PropertyDefinition → MARKUP_IDENTIFIER COLON Type [EQ RhsExpression]
  - fields: name: string, type: TypeSyntax, default?: ExpressionSyntax



RhsExpression (AST: ExpressionSyntax; see mappings below)
- RhsExpression → Element
- RhsExpression → Literal
- RhsExpression → InterpolationExpression

InterpolationExpression (AST: InterpolationExpressionSyntax)
- InterpolationExpression → LBRACE ValueExpression RBRACE
  - fields: expr: ExpressionSyntax

ValueExpression (AST: ExpressionSyntax; Pratt-parsed for operators)
- ValueExpression → Element
- ValueExpression → ValueIfExpression
- ValueExpression → ValueForExpression
- ValueExpression → ParenFunctionCall
- ValueExpression → ValueExpr

ValueExpr (parsed by Pratt; not a standalone AST node)
- Primaries (nud): Literal | IDENTIFIER | Unit | ParenthesizedExpression
- Postfix/infix handled via the operator table (including the conditional operator `?:` at precedence 20)

ParenFunctionCall (AST: ParenFunctionCallExpressionSyntax)
- ParenFunctionCall → ValueExpression LPAREN ParenFunctionCallArgumentListOpt RPAREN   (* parsed via Pratt entry at precedence 140; left-recursive form shown for clarity *)
  - fields: callee: ExpressionSyntax, args: ExpressionSyntax[]

ParenFunctionCallArgumentListOpt
- ParenFunctionCallArgumentListOpt → ParenFunctionCallArgumentList
- ParenFunctionCallArgumentListOpt → ε

ParenFunctionCallArgumentList
- ParenFunctionCallArgumentList → ValueExpression (COMMA ValueExpression)*

Unit (AST: UnitLiteralSyntax)
- Unit → LPAREN RPAREN
  - fields: (none)

ParenthesizedExpression (AST: ParenthesizedExpressionSyntax)
- ParenthesizedExpression → LPAREN ValueExpression RPAREN
  - fields: expr: ExpressionSyntax

ConditionalExpression (AST: ConditionalExpressionSyntax)
- ConditionalExpression → ValueExpression QMARK ValueExpression COLON ValueExpression  (parsed via Pratt entry at precedence 20; right-associative)
  - fields: condition: ExpressionSyntax, whenTrue: ExpressionSyntax, whenFalse: ExpressionSyntax

Literal (AST: LiteralExpressionSyntax)
- Literal → STRING_LITERAL | INT_LITERAL | REAL_LITERAL | HEX_LITERAL | BOOL_LITERAL | NULL_LITERAL
  - fields: kind: "string"|"int"|"real"|"hex"|"bool"|"null", value: token payload

ValueIfExpression (AST: ExpressionSyntax is a sum type)
- ValueIfExpression → ValueIfSimpleExpression        (ValueIfSimpleExpressionSyntax)
- ValueIfExpression → ValueIfMatchExpression         (ValueIfMatchExpressionSyntax)
- ValueIfExpression → ValueIfConditionListExpression (ValueIfConditionListExpressionSyntax)

ValueIfSimpleExpression (AST: ValueIfSimpleExpressionSyntax)
- ValueIfSimpleExpression → IF ValueExpression LBRACE ValueExpression RBRACE ValueIfElseClauseOpt
  - fields: condition: ExpressionSyntax, thenExpr: ExpressionSyntax, elseExpr?: ExpressionSyntax

ValueIfElseClauseOpt
- ValueIfElseClauseOpt → ELSE LBRACE ValueExpression RBRACE
- ValueIfElseClauseOpt → ε
  - fields (on ValueIfSimpleExpressionSyntax): elseExpr?: ExpressionSyntax

ValueIfMatchExpression (AST: ValueIfMatchExpressionSyntax)
- ValueIfMatchExpression → IF ValueExpression IS LBRACE ValueIfMatchArm+ ValueIfMatchElseOpt RBRACE
  - fields: scrutinee: ExpressionSyntax, arms: ValueIfMatchArmSyntax[], elseExpr?: ExpressionSyntax

ValueIfMatchArm (AST: ValueIfMatchArmSyntax)
- ValueIfMatchArm → Pattern (COMMA Pattern)* COLON ValueExpression
  - fields: patterns: PatternSyntax[], expr: ExpressionSyntax

ValueIfMatchElseOpt
- ValueIfMatchElseOpt → ELSE COLON ValueExpression
- ValueIfMatchElseOpt → ε
  - fields (on ValueIfMatchExpressionSyntax): elseExpr?: ExpressionSyntax

ValueIfConditionListExpression (AST: ValueIfConditionListExpressionSyntax)
- ValueIfConditionListExpression → IF LBRACE ValueIfConditionArm+ ValueIfConditionElseOpt RBRACE
  - fields: arms: ValueIfConditionArmSyntax[], elseExpr?: ExpressionSyntax

ValueIfConditionArm (AST: ValueIfConditionArmSyntax)
- ValueIfConditionArm → ValueExpression COLON ValueExpression
  - fields: condition: ExpressionSyntax, expr: ExpressionSyntax

ValueIfConditionElseOpt
- ValueIfConditionElseOpt → ELSE COLON ValueExpression
- ValueIfConditionElseOpt → ε
  - fields (on ValueIfConditionListExpressionSyntax): elseExpr?: ExpressionSyntax

ValueForExpression (AST: ValueForExpressionSyntax)
- ValueForExpression → FOR IDENTIFIER ForIndexOpt IN ValueExpression LBRACE ValueExpression RBRACE
  - fields: itemVar: string, indexVar?: string, iterable: ExpressionSyntax, body: ExpressionSyntax

ForIndexOpt
- ForIndexOpt → COMMA IDENTIFIER | ε
  - fields (on ValueForExpressionSyntax/MarkupForExpressionSyntax): itemVar: string, indexVar?: string

ElementsExpression (AST: MarkupListSyntax)
- ElementsExpression → ElementsExpressionItem+
  - fields: items: MarkupItemSyntax[]

- ElementsExpressionItem → Element                     (MarkupElementSyntax)
- ElementsExpressionItem → ElementsIfExpression        (MarkupIfSimpleExpressionSyntax | MarkupIfMatchExpressionSyntax | MarkupIfConditionListExpressionSyntax)
- ElementsExpressionItem → ElementsForExpression       (MarkupForExpressionSyntax)

ElementsIfExpression (AST: MarkupItemSyntax is a sum type)
- ElementsIfExpression → ElementsIfSimpleExpression        (MarkupIfSimpleExpressionSyntax)
- ElementsIfExpression → ElementsIfMatchExpression         (MarkupIfMatchExpressionSyntax)
- ElementsIfExpression → ElementsIfConditionListExpression (MarkupIfConditionListExpressionSyntax)

ElementsIfSimpleExpression (AST: MarkupIfSimpleExpressionSyntax)
- ElementsIfSimpleExpression → IF ValueExpression LBRACE ElementsExpression RBRACE ElementsIfElseClauseOpt
  - fields: condition: ExpressionSyntax, thenElements: MarkupListSyntax, elseElements?: MarkupListSyntax

ElementsIfElseClauseOpt
- ElementsIfElseClauseOpt → ELSE LBRACE ElementsExpression RBRACE
- ElementsIfElseClauseOpt → ε
  - fields (on MarkupIfSimpleExpressionSyntax): elseElements?: MarkupListSyntax

ElementsIfMatchExpression (AST: MarkupIfMatchExpressionSyntax)
- ElementsIfMatchExpression → IF ValueExpression IS LBRACE ElementsIfMatchArm+ ElementsIfMatchElseOpt RBRACE
  - fields: scrutinee: ExpressionSyntax, arms: MarkupIfMatchArmSyntax[], elseElements?: MarkupListSyntax

ElementsIfMatchArm (AST: MarkupIfMatchArmSyntax)
- ElementsIfMatchArm → Pattern (COMMA Pattern)* COLON ElementsExpression
  - fields: patterns: PatternSyntax[], elements: MarkupListSyntax

ElementsIfMatchElseOpt
- ElementsIfMatchElseOpt → ELSE COLON ElementsExpression
- ElementsIfMatchElseOpt → ε
  - fields (on MarkupIfMatchExpressionSyntax): elseElements?: MarkupListSyntax

ElementsIfConditionListExpression (AST: MarkupIfConditionListExpressionSyntax)
- ElementsIfConditionListExpression → IF LBRACE ElementsIfConditionArm+ ElementsIfConditionElseOpt RBRACE
  - fields: arms: MarkupIfConditionArmSyntax[], elseElements?: MarkupListSyntax

ElementsIfConditionArm (AST: MarkupIfConditionArmSyntax)
- ElementsIfConditionArm → ValueExpression COLON ElementsExpression
  - fields: condition: ExpressionSyntax, elements: MarkupListSyntax

ElementsIfConditionElseOpt
- ElementsIfConditionElseOpt → ELSE COLON ElementsExpression
- ElementsIfConditionElseOpt → ε
  - fields (on MarkupIfConditionListExpressionSyntax): elseElements?: MarkupListSyntax

ElementsForExpression (AST: MarkupForExpressionSyntax)
- ElementsForExpression → FOR IDENTIFIER ForIndexOpt IN ValueExpression LBRACE ElementsExpression RBRACE
  - fields: itemVar: string, indexVar?: string, iterable: ExpressionSyntax, body: MarkupListSyntax

Element (AST: MarkupElementSyntax is a sum type)
- Element → LT ElementName ElementSuffix

ElementSuffix (builds either Element or EmbedElement AST)
- ElementSuffix → PropertyList RegularElementTail
- ElementSuffix → COLON EmbedTextType EmbedElementTail

RegularElementTail (AST: ElementSyntax)
- RegularElementTail → SLASH GT
  - fields: name (from ElementName), props: PropertyListSyntax, children: []
- RegularElementTail → GT Content LT SLASH ElementName GT
  - fields: name (from ElementName), props: PropertyListSyntax, children: ElementContentSyntax.items

EmbedElementTail (AST: EmbedElementSyntax)
- EmbedElementTail → PropertyList GT EmbedContent LT SLASH ElementName GT
  - fields: name (from ElementName), textType (from EmbedTextType), mode: "parsed", props: PropertyListSyntax, content: EmbedContentSyntax.items
- EmbedElementTail → RAW PropertyList GT RawEmbedContent LT SLASH ElementName GT
  - fields: name (from ElementName), textType (from EmbedTextType), mode: "raw", props: PropertyListSyntax, content: RawEmbedContentSyntax.text

ElementName
- ElementName → QualifiedMarkupName

EmbedTextType
- EmbedTextType → IDENTIFIER
  - fields (on EmbedElement): textType: string

PropertyList (AST: PropertyListSyntax)
- PropertyList → PropertyListItem*
  - fields: items: PropertyListItemSyntax[]

PropertyListItem (AST: PropertyListItemSyntax is a sum type)
- PropertyListItem → PropertyValue                (PropertyValueSyntax)
- PropertyListItem → PropertyListIfExpression     (PropertyIfSimpleSyntax | PropertyIfMatchSyntax | PropertyIfConditionListSyntax)

PropertyValue (AST: PropertyValueSyntax)
- PropertyValue → QualifiedMarkupName EQ RhsExpression
  - fields: name: QualifiedMarkupNameSyntax, value: ExpressionSyntax

PropertyListIfExpression (AST: PropertyListItemSyntax is a sum type)
- PropertyListIfExpression → PropertyListIfSimpleExpression        (PropertyIfSimpleSyntax)
- PropertyListIfExpression → PropertyListIfMatchExpression         (PropertyIfMatchSyntax)
- PropertyListIfExpression → PropertyListIfConditionListExpression (PropertyIfConditionListSyntax)

PropertyListIfSimpleExpression (AST: PropertyIfSimpleSyntax)
- PropertyListIfSimpleExpression → IF ValueExpression LBRACE PropertyList RBRACE PropertyListIfElseClauseOpt
  - fields: condition: ExpressionSyntax, thenProps: PropertyListSyntax, elseProps?: PropertyListSyntax

PropertyListIfElseClauseOpt
- PropertyListIfElseClauseOpt → ELSE LBRACE PropertyList RBRACE
- PropertyListIfElseClauseOpt → ε

PropertyListIfMatchExpression (AST: PropertyIfMatchSyntax)
- PropertyListIfMatchExpression → IF ValueExpression IS LBRACE PropertyListIfMatchArm+ PropertyListIfMatchElseOpt RBRACE
  - fields: scrutinee: ExpressionSyntax, arms: PropertyIfMatchArmSyntax[], elseProps?: PropertyListSyntax

PropertyListIfMatchArm (AST: PropertyIfMatchArmSyntax)
- PropertyListIfMatchArm → Pattern (COMMA Pattern)* COLON PropertyList
  - fields: patterns: PatternSyntax[], props: PropertyListSyntax

PropertyListIfMatchElseOpt
- PropertyListIfMatchElseOpt → ELSE COLON PropertyList
- PropertyListIfMatchElseOpt → ε

PropertyListIfConditionListExpression (AST: PropertyIfConditionListSyntax)
- PropertyListIfConditionListExpression → IF LBRACE PropertyListIfConditionArm+ PropertyListIfConditionElseOpt RBRACE
  - fields: arms: PropertyIfConditionArmSyntax[], elseProps?: PropertyListSyntax

PropertyListIfConditionArm (AST: PropertyIfConditionArmSyntax)
- PropertyListIfConditionArm → ValueExpression COLON PropertyList
  - fields: condition: ExpressionSyntax, props: PropertyListSyntax

PropertyListIfConditionElseOpt
- PropertyListIfConditionElseOpt → ELSE COLON PropertyList
- PropertyListIfConditionElseOpt → ε

Content (AST: ElementContentSyntax is a sum type)
- Content → ElementsExpression
- Content → MixedContentExpression
  - fields: items: MarkupItemSyntax[]

MixedContentExpression (AST: MixedContentSyntax)
- MixedContentExpression → MixedContentItem+
  - fields: items: MixedContentItemSyntax[]

MixedContentItem (AST: MixedContentItemSyntax is a sum type)
- MixedContentItem → TextPart        (TextPartSyntax)
- MixedContentItem → Element         (MarkupElementSyntax)
- MixedContentItem → InterpolationExpression (InterpolationExpressionSyntax)

EmbedContent (AST: EmbedContentSyntax)
- EmbedContent → EmbedContentItem+
  - fields: items: EmbedContentItemSyntax[]

EmbedContentItem (AST: EmbedContentItemSyntax is a sum type)
- EmbedContentItem → TextRun         (TextRunSyntax)
- EmbedContentItem → InterpolationExpression (InterpolationExpressionSyntax)

RawEmbedContent (AST: RawEmbedContentSyntax)
- RawEmbedContent → RawTextRun
  - fields: text: string

RawTextRun (AST: TextRunSyntax)
- RawTextRun → RAW_TEXT_CHUNK+
  - fields: text: string (concatenated as-is)

TextPart (AST: TextPartSyntax)
- TextPart → TextRun
  - fields: text: string

TextRun (AST: TextRunSyntax)
- TextRun → (TEXT_CHUNK | ENTITY | ESCAPED_LBRACE | ESCAPED_RBRACE)+
  - fields: text: string  (concatenated, entities preserved or decoded by later phase)

QualifiedName (AST: QualifiedNameSyntax)
- QualifiedName → IDENTIFIER (DOT IDENTIFIER)*
  - fields: parts: string[]

QualifiedMarkupName (AST: QualifiedMarkupNameSyntax)
- QualifiedMarkupName → IDENTIFIER (DOT MARKUP_IDENTIFIER)*
  - fields: parts: string[]

Pattern (AST: PatternSyntax)
- Pattern → Literal
- Pattern → QualifiedName
  - fields: kind: "literal"|"name", value: LiteralExpressionSyntax|QualifiedNameSyntax

## AST Mapping Summary

This section lists the AST node types with fields for implementers.

- ModuleDefinitionSyntax: imports: ImportStatementSyntax[], members: ModuleMemberSyntax[], moduleElement?: MarkupElementSyntax (members and moduleElement can both be present)
- ModuleMemberSyntax: TypeDefinitionSyntax | ValueDefinitionSyntax | FunctionDefinitionSyntax
- ImportStatementSyntax: name: QualifiedNameSyntax
- TypeDefinitionSyntax: name: string, type: TypeSyntax
- ValueDefinitionSyntax: name: string, type?: TypeSyntax, value: ExpressionSyntax
- TypeSyntax: kind: "primitive"|"user", name: string (qualified), modifier?: "nullable"|"sequence"
- PrimitiveTypeSyntax: name: string
- UserTypeSyntax: name: QualifiedNameSyntax
 - FunctionDefinitionSyntax: ElementFunctionDefinitionSyntax | ParenFunctionDefinitionSyntax
 - ElementFunctionDefinitionSyntax: elementName: QualifiedMarkupNameSyntax, parameters: PropertyDefinitionSyntax[], returnType?: TypeSyntax, body: ExpressionSyntax
 - ParenFunctionDefinitionSyntax: name: string, parameters: PropertyDefinitionSyntax[], returnType?: TypeSyntax, body: ExpressionSyntax
 - PropertyDefinitionSyntax: name: string, type: TypeSyntax, default?: ExpressionSyntax
- ExpressionSyntax: union of MarkupElementSyntax | LiteralExpressionSyntax | IdentifierNameSyntax | ValueIfSimpleExpressionSyntax | ValueIfMatchExpressionSyntax | ValueIfConditionListExpressionSyntax | ValueForExpressionSyntax | ConditionalExpressionSyntax | ParenFunctionCallExpressionSyntax | MemberAccessExpressionSyntax | BinaryExpressionSyntax | PrefixUnaryExpressionSyntax | ParenthesizedExpressionSyntax | UnitLiteralSyntax
 - ParenFunctionCallExpressionSyntax: callee: ExpressionSyntax, args: ExpressionSyntax[]
 - MemberAccessExpressionSyntax: target: ExpressionSyntax, name: string (includes both property access and enum member access; distinguished during semantic analysis)
 - ConditionalExpressionSyntax: condition: ExpressionSyntax, whenTrue: ExpressionSyntax, whenFalse: ExpressionSyntax
 - BinaryExpressionSyntax: op: token, left: ExpressionSyntax, right: ExpressionSyntax
 - PrefixUnaryExpressionSyntax: op: token, expr: ExpressionSyntax
 - ParenthesizedExpressionSyntax: expr: ExpressionSyntax (may be elided)
 - UnitLiteralSyntax
 - LiteralExpressionSyntax: kind, value
 - IdentifierNameSyntax: name: string
- ValueIfSimpleExpressionSyntax: condition: ExpressionSyntax, thenExpr: ExpressionSyntax, elseExpr?: ExpressionSyntax
- ValueIfMatchExpressionSyntax: scrutinee: ExpressionSyntax, arms: ValueIfMatchArmSyntax[], elseExpr?: ExpressionSyntax
- ValueIfMatchArmSyntax: patterns: PatternSyntax[], expr: ExpressionSyntax
- ValueIfConditionListExpressionSyntax: arms: ValueIfConditionArmSyntax[], elseExpr?: ExpressionSyntax
- ValueIfConditionArmSyntax: condition: ExpressionSyntax, expr: ExpressionSyntax
- ValueForExpressionSyntax: itemVar: string, indexVar?: string, iterable: ExpressionSyntax, body: ExpressionSyntax
- MarkupListSyntax: items: MarkupItemSyntax[]
- MarkupItemSyntax: MarkupElementSyntax | MarkupIfSimpleExpressionSyntax | MarkupIfMatchExpressionSyntax | MarkupIfConditionListExpressionSyntax | MarkupForExpressionSyntax
- MarkupIfSimpleExpressionSyntax: condition: ExpressionSyntax, thenElements: MarkupListSyntax, elseElements?: MarkupListSyntax
- MarkupIfMatchExpressionSyntax: scrutinee: ExpressionSyntax, arms: MarkupIfMatchArmSyntax[], elseElements?: MarkupListSyntax
- MarkupIfMatchArmSyntax: patterns: PatternSyntax[], elements: MarkupListSyntax
- MarkupIfConditionListExpressionSyntax: arms: MarkupIfConditionArmSyntax[], elseElements?: MarkupListSyntax
- MarkupIfConditionArmSyntax: condition: ExpressionSyntax, elements: MarkupListSyntax
- MarkupForExpressionSyntax: itemVar: string, indexVar?: string, iterable: ExpressionSyntax, body: MarkupListSyntax
- MarkupElementSyntax: name: QualifiedMarkupNameSyntax, props: PropertyListSyntax, children: ElementContentSyntax (MarkupListSyntax or MixedContentSyntax)
- EmbedElementSyntax: name: QualifiedMarkupNameSyntax, textType: string, mode: "parsed"|"raw", props: PropertyListSyntax, content: EmbedContentSyntax|RawEmbedContentSyntax
- PropertyListSyntax: items: PropertyListItemSyntax[]
- PropertyListItemSyntax: PropertyValueSyntax | PropertyIfSimpleSyntax | PropertyIfMatchSyntax | PropertyIfConditionListSyntax
- PropertyValueSyntax: name: QualifiedMarkupNameSyntax, value: ExpressionSyntax
- PropertyIfSimpleSyntax: condition: ExpressionSyntax, thenProps: PropertyListSyntax, elseProps?: PropertyListSyntax
- PropertyIfMatchSyntax: scrutinee: ExpressionSyntax, arms: PropertyIfMatchArmSyntax[], elseProps?: PropertyListSyntax
- PropertyIfMatchArmSyntax: patterns: PatternSyntax[], props: PropertyListSyntax
- PropertyIfConditionListSyntax: arms: PropertyIfConditionArmSyntax[], elseProps?: PropertyListSyntax
- PropertyIfConditionArmSyntax: condition: ExpressionSyntax, props: PropertyListSyntax
- ElementContentSyntax: items: MarkupItemSyntax[] | MixedContentItemSyntax[]
- MixedContentSyntax: items: MixedContentItemSyntax[]
- MixedContentItemSyntax: kind: "text"|"element"|"interpolation", value: TextRunSyntax|MarkupElementSyntax|EmbedElementSyntax|InterpolationExpressionSyntax
- EmbedContentSyntax: items: (TextRunSyntax|InterpolationExpressionSyntax)[]
- RawEmbedContentSyntax: text: string
- TextPartSyntax: text: string
- TextRunSyntax: text: string
- InterpolationExpressionSyntax: expr: ExpressionSyntax
- QualifiedNameSyntax: parts: string[]
- QualifiedMarkupNameSyntax: parts: string[]
- PatternSyntax: kind: "literal"|"name", value: LiteralExpressionSyntax|QualifiedNameSyntax

## Disambiguation and Lookahead Notes

- ValueExpression branch selection:
  - If next token is LT → Element
  - If next token is IF → ValueIfExpression
  - If next token is FOR → ValueForExpression
  - Otherwise → ValueExpr (Pratt)
- ElementsExpression item selection:
  - If next token is LT → Element
  - If next token ∈ {IF, FOR} → the corresponding Elements* form
- IfMatch scrutinee (value/elements/property variants):
  - After IF, parse a required ValueExpression before IS as the scrutinee
  - Condition-list form begins directly with LBRACE and never has a scrutinee
- Element is left-factored: after LT ElementName, COLON selects the embed branch; otherwise parse PropertyList and choose SLASH GT (self-closing) or GT … LT SLASH ElementName GT using lookahead at SLASH vs GT.
- MemberAccess handles both property/field access and enum member access:
  - All `target.name` expressions parse uniformly as MemberAccessExpressionSyntax
  - Semantic analysis resolves the target expression to determine interpretation:
    - If target resolves to an enum type → enum member access (verify name is valid enum member)
    - If target resolves to a value → property/field access (verify name is valid property/field on target's type)
  - Examples: `Status.Active` (if Status is enum type), `obj.field` (if obj is value), `foo.bar` (ambiguous at parse time, resolved during type checking)

## Validation Rules (post-parse)

- Element closing tag name must match opening ElementName.
- EmbedElement closing tag name must match opening ElementName.
- PropertyDefinition names within a single FunctionDefinition should be unique.
- Type modifiers: at most one of QMARK or LBRACK RBRACK.
- Switch expressions (property variants): at least one case; patterns per case must be non-empty.
- ValueIfMatchExpression / ElementsIfMatchExpression / PropertyListIfMatchExpression: at least one pattern arm; each arm requires ≥1 pattern.
- ValueIfConditionListExpression / ElementsIfConditionListExpression / PropertyListIfConditionListExpression: at least one condition arm.

## Notes and Gaps

- Pattern is limited to constant-like forms (Literal or QualifiedName). Extend as needed.
- Entities in TextRun are preserved as ENTITY tokens; decoding can be a later phase.
- ParenthesizedExpression may be elided in AST after parsing.
