// Parse only the selected key as data, following Bombyx's env-file contract.
export function secretValue(env, key) {
  let value;
  for (const line of env.split("\n")) {
    const match = line
      .replace(/\r$/, "")
      .match(/^\s*(?:export\s+)?([A-Z_][A-Z0-9_]*)\s*=\s*(.*)$/);
    if (!match || match[1] !== key) continue;
    const raw = match[2].trim();
    if (raw.startsWith('"') || raw.startsWith("'")) {
      const end = raw.indexOf(raw[0], 1);
      if (end < 0) throw Error("quote");
      value = raw.slice(1, end);
    } else {
      value = raw.replace(/\s+#.*$/, "").trim();
    }
  }
  return value;
}
