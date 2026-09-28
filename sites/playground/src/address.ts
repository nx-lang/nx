/**
 * Keeping the address in step with the source.
 *
 * <para>An edit reaches the address after a pause in typing, replacing the current history entry, so
 * a reload keeps the visitor's work without a history entry per keystroke. Choosing an example adds
 * an entry, and first writes any edit still waiting for its pause into the entry being left, so Back
 * returns to it. The address follows the edit, not the evaluation: a source that crashed the compiler
 * is still the visitor's work.</para>
 *
 * <para>Kept apart from React and the browser, with the history, the encoder and the timer passed
 * in, so the policy can be tested under Node.</para>
 */
import { pathForExample, pathForPayload } from "./routes.ts";

/** The part of `window.history` the keeper uses. */
export interface HistoryLike {
  pushState(data: unknown, unused: string, url: string): void;
  replaceState(data: unknown, unused: string, url: string): void;
}

export interface AddressKeeperOptions {
  readonly history: HistoryLike;
  /** The site's prefix, `/play`. */
  readonly root: string;
  /** Encodes source as a `#code=` payload. */
  readonly encode: (source: string) => Promise<string>;
  /** How long typing must pause before the address is updated. */
  readonly delayMs?: number;
  readonly setTimer?: (run: () => void, ms: number) => unknown;
  readonly clearTimer?: (timer: unknown) => void;
}

export interface AddressKeeper {
  /** The visitor typed: the address carries `source` once typing pauses. */
  edited(source: string): void;
  /** The visitor chose an example: keep any waiting edit in the current entry, then add one. */
  choose(id: string): Promise<void>;
  /**
   * The address changed under the page (Back or Forward): a waiting edit belonged to the entry that
   * was left, so it is dropped rather than written into this one.
   */
  navigated(): void;
}

export function createAddressKeeper(options: AddressKeeperOptions): AddressKeeper {
  const delayMs = options.delayMs ?? 350;
  const setTimer = options.setTimer ?? ((run, ms) => setTimeout(run, ms));
  const clearTimer = options.clearTimer ?? ((timer) => clearTimeout(timer as ReturnType<typeof setTimeout>));
  let timer: unknown = null;
  // The latest edit the address does not carry yet: waiting for its pause, or being encoded.
  let unwritten: string | null = null;
  // Bumped on every change, so an encoding that finishes after a newer change is dropped.
  let generation = 0;

  const write = async (source: string, current: number) => {
    const payload = await options.encode(source);
    if (current === generation) {
      options.history.replaceState(null, "", pathForPayload(payload, options.root));
      if (unwritten === source) {
        unwritten = null;
      }
    }
  };

  const stopTimer = () => {
    if (timer !== null) {
      clearTimer(timer);
      timer = null;
    }
  };

  return {
    edited(source) {
      stopTimer();
      unwritten = source;
      const current = ++generation;
      timer = setTimer(() => {
        timer = null;
        void write(source, current);
      }, delayMs);
    },
    async choose(id) {
      stopTimer();
      const current = ++generation;
      if (unwritten !== null) {
        await write(unwritten, current);
      }
      if (current === generation) {
        options.history.pushState(null, "", pathForExample(id, options.root));
      }
    },
    navigated() {
      stopTimer();
      unwritten = null;
      generation += 1;
    },
  };
}
