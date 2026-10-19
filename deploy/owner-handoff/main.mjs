// Small privileged entry point: fixed private configuration, generic errors.
import { readConfig } from "./config.mjs";
import { startHandoff } from "./runtime.mjs";

try {
  if (process.getuid() !== 0 || process.argv.length !== 2)
    throw new Error("Invalid invocation.");
  await startHandoff(readConfig("/etc/silkboard-owner-handoff.json", true));
} catch {
  process.stderr.write("Private owner handoff could not start.\n");
  process.exitCode = 1;
}
