// SPDX-License-Identifier: GPL-3.0-or-later

const PALETTE: Array<[string, string]> = [
  ["#ff9f0a", "#ff6a00"],
  ["#30b0c7", "#007aa5"],
  ["#5e5ce6", "#3634a3"],
  ["#bf5af2", "#8944ab"],
  ["#ff6482", "#d70040"],
  ["#34c759", "#1f8f3e"],
  ["#64d2ff", "#1a8fc0"],
  ["#a2845e", "#7a5c3a"],
];

function hash(text: string): number {
  let value = 0;
  for (let i = 0; i < text.length; i += 1) value = (value * 31 + text.charCodeAt(i)) >>> 0;
  return value;
}

/** Up to two initials from a name; a person with only a number or address gets none. */
export function initialsFor(name: string): string {
  const words = name
    .trim()
    .split(/\s+/)
    .filter((word) => /^\p{L}/u.test(word));
  if (words.length === 0) return "";
  const letters = words.length === 1 ? [words[0]] : [words[0], words[words.length - 1]];
  return letters.map((word) => Array.from(word)[0].toUpperCase()).join("");
}

/** A round contact picture: initials on a colour that is stable for the same name. */
export function Avatar({ name, size = 28 }: { name: string; size?: number }) {
  const [from, to] = PALETTE[hash(name) % PALETTE.length];
  const initials = initialsFor(name);
  return (
    <span
      className="avatar"
      aria-hidden="true"
      style={{
        width: size,
        height: size,
        fontSize: Math.round(size * 0.4),
        background: `linear-gradient(160deg, ${from}, ${to})`,
      }}
    >
      {initials || (
        <svg viewBox="0 0 24 24" width={size * 0.6} height={size * 0.6} fill="currentColor">
          <circle cx="12" cy="8.5" r="4" />
          <path d="M4.5 20c.6-4 3.6-6 7.5-6s6.9 2 7.5 6Z" />
        </svg>
      )}
    </span>
  );
}
