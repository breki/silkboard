import fs from "node:fs";
import path from "node:path";
import { randomBytes } from "node:crypto";

export function directory(dir) {
  if (!path.isAbsolute(dir) || path.resolve(dir) !== dir) throw Error("path");
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { mode: 0o700 });
  const stat = fs.lstatSync(dir);
  if (
    !stat.isDirectory() ||
    stat.uid !== process.getuid() ||
    (stat.mode & 0o777) !== 0o700 ||
    fs.realpathSync(dir) !== dir
  )
    throw Error("directory");
}
export function file(filePath) {
  const fd = fs.openSync(
    filePath,
    fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW | fs.constants.O_NONBLOCK,
  );
  try {
    const stat = fs.fstatSync(fd);
    if (
      !stat.isFile() ||
      stat.uid !== process.getuid() ||
      stat.nlink !== 1 ||
      (stat.mode & 0o777) !== 0o600 ||
      stat.size > 65536 ||
      fs.realpathSync(filePath) !== filePath
    )
      throw Error("file");
    return fs.readFileSync(fd, "utf8");
  } finally {
    fs.closeSync(fd);
  }
}
export function replace(filePath, value) {
  if (fs.lstatSync(filePath, { throwIfNoEntry: false })) file(filePath);
  const temp = path.join(
    path.dirname(filePath),
    ".silkboard-" + randomBytes(12).toString("hex"),
  );
  try {
    const fd = fs.openSync(
      temp,
      fs.constants.O_WRONLY |
        fs.constants.O_CREAT |
        fs.constants.O_EXCL |
        fs.constants.O_NOFOLLOW,
      0o600,
    );
    try {
      fs.writeFileSync(fd, value);
      fs.fsyncSync(fd);
    } finally {
      fs.closeSync(fd);
    }
    fs.renameSync(temp, filePath);
    const parent = fs.openSync(path.dirname(filePath), fs.constants.O_RDONLY);
    try {
      fs.fsyncSync(parent);
    } finally {
      fs.closeSync(parent);
    }
  } finally {
    fs.rmSync(temp, { force: true });
  }
}
export function paths(home) {
  if (
    !path.isAbsolute(home) ||
    fs.realpathSync(home) !== home ||
    fs.statSync(home).uid !== process.getuid() ||
    (fs.statSync(home).mode & 0o022) !== 0
  )
    throw Error("home");
  const configParent = path.join(home, ".config");
  if (!fs.existsSync(configParent)) fs.mkdirSync(configParent, { mode: 0o700 });
  const stat = fs.lstatSync(configParent);
  if (
    !stat.isDirectory() ||
    stat.uid !== process.getuid() ||
    (stat.mode & 0o022) !== 0 ||
    fs.realpathSync(configParent) !== configParent
  )
    throw Error("config parent");
  const configDir = path.join(configParent, "silkboard-publish");
  directory(configDir);
  return {
    configDir,
    config: path.join(configDir, "config.json"),
    token: path.join(configDir, "publisher.token"),
  };
}
