/**
 * Label abbreviation (spec §7 "A" toggle): "Application Support" → "AS",
 * "node_modules" → "NM". Split on separators; take per-word initials up
 * to `budget`; a single word that fits the budget stays whole, a longer
 * one truncates to `budget` chars with an ellipsis; budget 0 → "".
 */
export function abbreviate(name: string, budget = 2): string {
  if (budget <= 0) return "";
  const words = name.split(/[\s_-]+/).filter(Boolean);
  if (words.length <= 1) {
    if (name.length <= budget) return name;
    return `${name.slice(0, Math.max(0, budget - 1))}…`;
  }
  return words
    .slice(0, Math.max(2, budget))
    .map((w) => w[0]?.toUpperCase() ?? "")
    .join("");
}
