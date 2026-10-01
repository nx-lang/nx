/**
 * Opens the completion list's details pane the first time it can show something.
 *
 * <para>A completion's documentation and full detail are in a pane beside the list that Monaco
 * keeps closed until the visitor presses Ctrl+Space again or clicks an item's chevron, and a first
 * visit would not know to. Monaco has no option for the pane's initial state, so this opens it once
 * through the suggest widget, as that gesture would. After that it is the visitor's: Monaco
 * remembers the pane's state for the session, and closing it is respected.</para>
 *
 * <para>The widget is reached through internals Monaco does not type: its suggest controller's
 * `widget`, and the widget's `onDidShow`, `toggleDetails` and `_isDetailsVisible`. A Monaco release
 * that changes their shape leaves the pane closed, which is Monaco's own default, rather than
 * failing.</para>
 */
import type * as monaco from "monaco-editor";

interface SuggestWidgetInternals {
  onDidShow(listener: () => void): monaco.IDisposable;
  toggleDetails(): void;
  _isDetailsVisible(): boolean;
}

interface SuggestControllerInternals {
  widget?: { value?: Partial<SuggestWidgetInternals> };
}

const SUGGEST_CONTROLLER = "editor.contrib.suggestController";

export function openSuggestionDetailsOnce(editor: monaco.editor.IStandaloneCodeEditor): monaco.IDisposable {
  const controller = editor.getContribution(SUGGEST_CONTROLLER) as SuggestControllerInternals | null;
  const widget = controller?.widget?.value;
  if (
    typeof widget?.onDidShow !== "function" ||
    typeof widget.toggleDetails !== "function" ||
    typeof widget._isDetailsVisible !== "function"
  ) {
    return { dispose() {} };
  }
  const { toggleDetails, _isDetailsVisible: isDetailsVisible } = widget as SuggestWidgetInternals;
  const subscription = widget.onDidShow(() => {
    if (!isDetailsVisible.call(widget)) {
      // Opens only when the focused item has a detail or documentation to show, so a list of bare
      // keywords leaves it for the next list.
      toggleDetails.call(widget);
    }
    if (isDetailsVisible.call(widget)) {
      subscription.dispose();
    }
  });
  return subscription;
}
