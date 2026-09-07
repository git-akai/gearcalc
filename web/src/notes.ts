/**
 * What a field says about itself under its own box.
 *
 * Two panels grew the same idea separately: a slot under a field holding every
 * sentence it might say — the bound it must stay inside, the reason it is what
 * it is, the complaint when it is wrong — with one of them visible. They are
 * stacked rather than swapped so the slot takes the height of the tallest and
 * nothing on the page moves when the visible one changes.
 *
 * One builder and one component now, because two copies of a convention are a
 * convention that will eventually disagree with itself: the train panel's had
 * become a snippet while the gear panel's stayed written out at each call, and
 * the gear panel's simple notes were plain `<small>` elements that ran to the
 * row's full width instead of stopping where the numbers stop.
 */
export interface Notes {
  /** Every sentence this field might show.
   *
   *  The first is a non-breaking space, which is what reserves the slot's height
   *  on a field that has nothing to say right now — an error arriving as you
   *  type is exactly the case that used to shift the whole column. A plain space
   *  collapses and would reserve nothing. */
  all: { text: string; err?: boolean }[];
  /** Which of them is visible. */
  shown: number;
}

/**
 * A field's notes: what it normally says, and what it says when it is wrong.
 *
 * The complaint wins when there is one, since a reader who has typed something
 * inadmissible is owed that before the reason behind the bound.
 */
export function notes(range: string | null, bad: string | null): Notes {
  const all: Notes["all"] = [{ text: "\u00a0" }];
  if (range) all.push({ text: range });
  if (bad) all.push({ text: bad, err: true });
  return { all, shown: bad ? all.length - 1 : range ? 1 : 0 };
}
