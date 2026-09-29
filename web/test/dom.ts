// Reading and typing into a mounted panel the way a reader does: by the
// words on screen, through the catalogue, never by a component's internals.
import { flushSync, mount, unmount, type Component } from "svelte";
import { t } from "../src/core";

/** Mount `component` with `props`, and a way to take it down again. */
export function mounted<P extends Record<string, unknown>>(component: Component<P>, props: P) {
  const target = document.body.appendChild(document.createElement("div"));
  const app = mount(component, { target, props });
  flushSync();
  return {
    target,
    done() {
      unmount(app);
      target.remove();
    },
  };
}

/** The row whose name reads `t(key)`. */
export function row(root: ParentNode, key: string): HTMLLabelElement {
  const name = t(key);
  const found = [...root.querySelectorAll("label")].find(
    (l) => l.querySelector(":scope > span")?.textContent?.trim() === name,
  );
  if (!found) throw new Error(`no row named ${key} (${name})`);
  return found;
}

/** The number box in the row named `t(key)`. */
export function box(root: ParentNode, key: string): HTMLInputElement {
  const input = row(root, key).querySelector<HTMLInputElement>('input[type="number"]');
  if (!input) throw new Error(`row ${key} has no number box`);
  return input;
}

/** Type `text` into `input` as one edit, and let the panel settle. */
export function type(input: HTMLInputElement, text: string) {
  input.focus();
  input.value = text;
  input.dispatchEvent(new Event("input", { bubbles: true }));
  flushSync();
}

/** Leave `input`, as tabbing out does. */
export function leave(input: HTMLInputElement) {
  input.blur();
  input.dispatchEvent(new Event("change", { bubbles: true }));
  flushSync();
}

/** The complaint the row named `t(key)` is showing, or null. */
export function complaint(root: ParentNode, key: string): string | null {
  const err = row(root, key).querySelector("small.err:not(.hidden)");
  return err ? (err.textContent ?? "").trim() : null;
}
