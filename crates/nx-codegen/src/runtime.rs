use crate::options::CodegenTarget;

pub fn runtime_helper_source(target: CodegenTarget) -> String {
    match target {
        CodegenTarget::TypeScript => typescript_runtime(),
        CodegenTarget::JavaScript => javascript_runtime(),
    }
}

fn typescript_runtime() -> String {
    r#"export type NxValue =
  | null
  | boolean
  | number
  | string
  | readonly NxValue[]
  | { readonly [key: string]: NxValue };

export function nxElement(
  tag: string,
  properties: Record<string, NxValue>,
  content: readonly NxValue[] = [],
): NxValue {
  const output: Record<string, NxValue> = { ...properties };
  if (content.length === 1) {
    output.content = content[0];
  } else if (content.length > 1) {
    output.content = content;
  }
  return { $type: tag, ...output };
}

export function nxRuntimeError(message: string): never {
  throw new Error(message);
}
"#
    .to_string()
}

fn javascript_runtime() -> String {
    r#"export function nxElement(tag, properties, content = []) {
  const output = { ...properties };
  if (content.length === 1) {
    output.content = content[0];
  } else if (content.length > 1) {
    output.content = content;
  }
  return { $type: tag, ...output };
}

export function nxRuntimeError(message) {
  throw new Error(message);
}
"#
    .to_string()
}
