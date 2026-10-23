import { refreshSecrets } from "./refresh-secrets.mjs";
const failures = refreshSecrets(process.env.HOME, process.env.BOMBYX_ENV_FILE);
for (const message of failures) console.error(message);
process.exitCode = failures.length ? 1 : 0;
