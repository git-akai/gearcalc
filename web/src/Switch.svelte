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
    /* Full contrast in both states, as the actuation control's segments are:
       what says a switch is on is the fill behind it, not the strength of its
       text. Muted-when-off reads as a smaller word rather than an unlit one,
       and next to a segmented control that never mutes it looked like a
       different size of type. It is the same size — 0.75rem, both of them. */
    color: var(--fg);
    cursor: pointer;
    /* Where it sits, the row decides: nothing is asserted here, so a row that
       wants it at one edge says so with `justify-items` and one that gives it a
       column of its own gets it at that column's width. */
  }
  button:hover {
    background: var(--hover);
  }
  /* The `auto` toggles keep the muted, smaller styling they have always had:
     they name a mode rather than a field, and stay out of the way of the number
     they qualify. */
  button.small {
    font-size: 0.7rem;
    padding: 0.1rem 0.35rem;
    color: var(--muted);
  }
  button.on {
    background: var(--selected);
    color: var(--fg);
  }
</style>
