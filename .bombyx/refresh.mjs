import fs from "node:fs";
import { file, replace, paths } from "./private-files.mjs";
import { secretValue } from "./secret-value.mjs";
export function refresh(home, envFile) {
  let destination;
  try {
    const p = paths(home);
    const config = JSON.parse(file(p.config));
    if (config.token_file !== p.token) throw Error("destination");
    destination = p.token;
    const value = secretValue(file(envFile), "SILKBOARD_PUBLISH_TOKEN");
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
