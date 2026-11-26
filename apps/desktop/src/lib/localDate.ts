/**
 * Today's calendar date as the person sees it, not in UTC. Screens work out months with UTC
 * date maths, so this returns a Date at noon UTC on the local calendar day. Using
 * `new Date()` directly would switch to tomorrow (and to next month on the last evening of a
 * month) for anyone west of Greenwich.
 */
export function localToday(now: Date = new Date()): Date {
  return new Date(Date.UTC(now.getFullYear(), now.getMonth(), now.getDate(), 12));
}

/** The local calendar day as YYYY-MM-DD. */
export function localIso(now: Date = new Date()): string {
  return localToday(now).toISOString().slice(0, 10);
}
