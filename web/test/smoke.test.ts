// The runner itself: a panel mounts against the real core and draws a number.
import { flushSync, mount, unmount } from "svelte";
import { expect, test } from "vitest";
import GearPanel from "../src/GearPanel.svelte";
import TrainPanel from "../src/TrainPanel.svelte";
import { trains, workspace } from "../src/state.svelte";

test("the gear panel mounts and shows the tab's module", () => {
  const target = document.body.appendChild(document.createElement("div"));
  const app = mount(GearPanel, { target, props: { tab: workspace.tabs[0] } });
  flushSync();
  const boxes = [...target.querySelectorAll<HTMLInputElement>('input[type="number"]')];
  expect(boxes.map((b) => b.value)).toContain(String(workspace.tabs[0].params.module));
  unmount(app);
  target.remove();
});

test("the train panel mounts and names its members", () => {
  const target = document.body.appendChild(document.createElement("div"));
  const app = mount(TrainPanel, { target, props: { tab: trains.tabs[0] } });
  flushSync();
  expect(target.textContent).not.toContain("ui.train_boundary_failed");
  expect(target.querySelectorAll("button").length).toBeGreaterThan(0);
  unmount(app);
  target.remove();
});
