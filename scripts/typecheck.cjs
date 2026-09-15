// Bun loads CommonJS without vue-tsc's fs.readFileSync interception.
// Compile Volar's transformed TypeScript CLI explicitly under Bun instead.
const fs = require("node:fs");
const Module = require("node:module");
const path = require("node:path");
const core = require("@vue/language-core");
const volar = require("@volar/typescript/lib/quickstart/runTsc");
volar.getLanguagePlugins = (ts, options) => {
  const config = options.options.configFilePath;
  const vueOptions = core.createParsedCommandLine(
    ts,
    ts.sys,
    config.replaceAll("\\", "/"),
  ).vueOptions;
  return {
    languagePlugins: [
      core.createVueLanguagePlugin(ts, options.options, vueOptions, (id) => id),
    ],
  };
};
const filename = require.resolve("typescript/lib/_tsc.js");
const source = volar.transformTscContent(
  fs.readFileSync(filename, "utf8"),
  require.resolve("@volar/typescript/lib/node/proxyCreateProgram"),
  [".vue"],
  [],
);
const compiled = new Module(filename, module);
compiled.filename = filename;
compiled.paths = Module._nodeModulePaths(path.dirname(filename));
compiled._compile(source, filename);
