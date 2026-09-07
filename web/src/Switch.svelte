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
    small = false,
  }: {
    label: string;
    on: boolean;
    set: (v: boolean) => void;
    title?: string;
    /** The `auto` toggles, which sit inside a field row rather than being one:
     *  they name a mode rather than the field, and stay out of the way of the
     *  number they qualify. */
    small?: boolean;
  } = $props();
</script>

<button type="button" class:on class:small aria-pressed={on} onclick={() => set(!on)} {title}
  >{label}</button
>

<style>
  /* Sized as the actuation control's segments are: a switch that carries its
     own name is a field in its own right, and reads like one. */
  button {
    font: inherit;
    font-size: 0.75rem;
    padding: 0.25rem 0.6rem;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: none;
    color: var(--muted);
    cursor: pointer;
    /* As wide as its own word, never as wide as the cell it lands in. Where it
       sits the row is what decides: an `auto` toggle keeps to the left of the
       number it qualifies, and a switch that names a field is put at the right
       edge the inputs share. */
    justify-self: inherit;
  }
  button:hover {
    background: var(--hover);
  }
  button.small {
    font-size: 0.7rem;
    padding: 0.1rem 0.35rem;
  }
  button.on {
    background: var(--selected);
    color: var(--fg);
  }
</style>
