// Minimal ambient declaration of the Node `process` global so this CLI builds
// with zero dependencies (no @types/node). `fetch`, `Response`, `console`,
// `setTimeout` come from the DOM lib referenced in tsconfig.
declare const process: {
  argv: string[];
  env: Record<string, string | undefined>;
  exit(code?: number): never;
};
