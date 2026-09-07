<!-- **A field's note, under the field.**

     Placed here rather than by whichever row it lands in, because where it ends
     is a property of the note and not of the container: every field row in the
     application keeps a trailing cell for its unit, and a note ends where the
     *input* does, one cell short of the row. `1 / -2` is that in every template
     at once — three columns or four, a label column or an `auto` toggle beside
     it — and it is why this is a component rather than a snippet each panel
     keeps its own copy of, each with its own column count written out.

     In a row that is not a grid at all, the eccentric drive's switch rows among
     them, the column has no meaning and the row's own alignment carries it. -->
<script lang="ts">
  import type { Notes } from "./notes";

  let { notes }: { notes: Notes } = $props();
</script>

<span class="note">
  {#each notes.all as note, i (i)}
    <small class:err={note.err} class:hidden={i !== notes.shown}>{note.text}</small>
  {/each}
</span>

<style>
  .note {
    grid-column: 1 / -2;
    display: grid;
    text-align: right;
  }
  /* All in one cell, so the slot is as tall as the tallest and nothing moves
     when the visible one changes. */
  small {
    grid-area: 1 / 1;
    font-size: 0.72rem;
    color: var(--muted);
  }
  small.err {
    color: var(--warn);
  }
  small.hidden {
    visibility: hidden;
  }
</style>
