<!-- **A field's note, under the field.**

     Placed here rather than by whichever row it lands in, because where it ends
     is a property of the note and not of the container. It runs the whole row —
     the same right boundary the input boxes and their units are spaced off —
     which is a sentence's worth of extra room over stopping where the boxes
     stop, and a note is the one thing in a row that can want it. `1 / -1` is
     that in every template at once, three columns or four, and it is why this
     is a component rather than a snippet each panel keeps its own copy of with
     its own column count written out.

     In a row that is not a grid at all — a switch row — the column has no
     meaning, and the row keeps its inset off the control rather than off
     itself so the note still reaches the edge. -->
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
    grid-column: 1 / -1;
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
