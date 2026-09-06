<!-- **One switch, wherever something is on or off.**

     Booleans were checkboxes and modes were buttons, which made two controls
     out of one idea: a checkbox is a native widget that answers to the browser
     rather than to this palette, it needs a label beside it to say what it is,
     and next to the `auto` buttons and the actuation segments it read as a
     different kind of thing. This is that same button — lit when on, plain when
     off — so a boolean looks like every other switch in the application.

     `label` is the text on the button. Where the switch sits in a field row of
     its own the row's label says what it is and the button says the state;
     where it stands alone the button carries the name. -->
<script lang="ts">
  let {
    label,
    on,
    set,
    title,
  }: { label: string; on: boolean; set: (v: boolean) => void; title?: string } = $props();
</script>

<button type="button" class:on aria-pressed={on} onclick={() => set(!on)} {title}>{label}</button>

<style>
  button {
    font: inherit;
    font-size: 0.7rem;
    padding: 0.1rem 0.35rem;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: none;
    color: var(--muted);
    cursor: pointer;
    /* A switch is as wide as its own word, not as wide as the column it sits
       in: in a field row it takes the trailing cell that the `auto` buttons
       take, and standing alone it takes only what it needs. */
    justify-self: start;
  }
  button:hover {
    background: var(--hover);
  }
  button.on {
    background: var(--selected);
    color: var(--fg);
  }
</style>
