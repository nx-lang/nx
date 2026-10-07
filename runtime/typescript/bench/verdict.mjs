/**
 * How the comparison judges a step from the ratios of its rounds, each the working tree's time
 * over the base's.
 *
 * One round's ratio is noisy: between two runs of one build, one warm ratio in twelve is more
 * than 10% from 1 and a few are twice or half. The median of seven is not: over 1,200 steps of a
 * build compared with itself, on a desktop and on GitHub runners, it stayed between 0.90 and 1.10
 * and nearly always within 5% of 1. So a step is named when the median of its ratios is beyond
 * the threshold and three quarters of its rounds are beyond half of it the same way. Of seven
 * rounds that is six, so one or two wild rounds do not name a step and one does not hide a real
 * slowdown; two that go the other way do. At a threshold of 7% that rule named
 * none of those 1,200, and names 84% of them when every ratio is made 10% larger and 96% when
 * 15%. A cold time is one sample a round, and at 20% the rule named none of 340.
 */
import { median } from "./sample.mjs";

/** How much of the threshold a round must be beyond to count as agreeing with the median. */
const AGREEMENT = 0.5;

/**
 * `"slower"`, `"faster"` or `""` for `ratios`: slower when their median is above `1 + threshold`
 * and at least three quarters of them are above `1 + 0.5 * threshold`, and faster by the same
 * measure the other way.
 */
export function verdict(ratios, threshold) {
  const needed = Math.ceil(ratios.length * 0.75);
  const middle = median(ratios);
  const margin = 1 + threshold * AGREEMENT;
  if (middle > 1 + threshold && ratios.filter((ratio) => ratio > margin).length >= needed) {
    return "slower";
  }
  if (middle < 1 / (1 + threshold) && ratios.filter((ratio) => ratio < 1 / margin).length >= needed) {
    return "faster";
  }
  return "";
}
