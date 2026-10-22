import fs from "node:fs";
import { file, replace, paths } from "./private-files.mjs";
export function refresh(home, envFile) {
  let destination;
  try {
    const p = paths(home);
    const config = JSON.parse(file(p.config));
    if (config.token_file !== p.token) throw Error("destination");
    destination = p.token;
    const env = file(envFile);
    let value;
    for (const line of env.split("\n")) {
      const match = line
        .replace(/\r$/, "")
        .match(/^\s*(?:export\s+)?SILKBOARD_PUBLISH_TOKEN\s*=\s*(.*)$/);
      if (!match) continue;
      const raw = match[1].trim();
      if (raw.startsWith('"') || raw.startsWith("'")) {
        const end = raw.indexOf(raw[0], 1);
        if (end < 0) throw Error("quote");
        value = raw.slice(1, end);
      } else {
        value = raw.replace(/\s+#.*$/, "").trim();
      }
    }
    if (!/^[a-f0-9]{64}$/.test(value ?? "")) throw Error("token");
    replace(destination, value + "\n");
  } catch (_) {
    if (destination) {
      try {
        file(destination);
        fs.unlinkSync(destination);
      } catch (_) {
        /* Never follow an unsafe destination. */
      }
    }
    throw Error(
      "Silkboard publisher secret refresh failed; check private input/configuration.",
    );
  }
}
