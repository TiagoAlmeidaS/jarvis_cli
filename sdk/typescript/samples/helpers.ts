import path from "node:path";

export function jarvisPathOverride() {
  return (
    process.env.JARVIS_EXECUTABLE ??
    path.join(process.cwd(), "..", "..", "jarvis-rs", "target", "debug", "jarvis")
  );
}
