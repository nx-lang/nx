/** What a model-facing tool name must match: inside what every provider accepts, with `__` free. */
export const NX_AGENT_TOOL_NAME_PATTERN = /^[a-z][a-z0-9_]{0,63}$/;

/** Whether `name` can be a model-facing tool name. */
export function isToolName(name: string): boolean {
  return NX_AGENT_TOOL_NAME_PATTERN.test(name);
}

function isUpper(character: string | undefined): boolean {
  return character !== undefined && character >= "A" && character <= "Z";
}

function isLower(character: string | undefined): boolean {
  return character !== undefined && character >= "a" && character <= "z";
}

function isDigit(character: string | undefined): boolean {
  return character !== undefined && character >= "0" && character <= "9";
}

/**
 * An NX name in snake_case, the way the package names a tool after its function: `findPlans` is
 * `find_plans`, `HTTPStatusFor` is `http_status_for` and `PlanRow` is `plan_row`.
 *
 * <para>A word begins at an upper-case letter that follows a lower-case letter or a digit, and at
 * the last upper-case letter of a run when a lower-case letter follows it. Every character that is
 * not an ASCII letter or digit ends a word and is dropped. The result is not checked against the
 * tool name pattern: a name that begins with a digit, or holds no ASCII letter, converts to
 * something the pattern refuses.</para>
 */
export function toSnakeCase(name: string): string {
  const words: string[] = [];
  let word = "";
  for (let index = 0; index < name.length; index += 1) {
    const character = name[index]!;
    const isLetterOrDigit = isUpper(character) || isLower(character) || isDigit(character);
    const previous = name[index - 1];
    const startsWord =
      isUpper(character) &&
      (isLower(previous) || isDigit(previous) || (isUpper(previous) && isLower(name[index + 1])));
    if ((!isLetterOrDigit || startsWord) && word !== "") {
      words.push(word);
      word = "";
    }
    if (isLetterOrDigit) {
      word += character.toLowerCase();
    }
  }
  if (word !== "") {
    words.push(word);
  }
  return words.join("_");
}
