/**
 * WASI is enabled through asconfig.json in post 0.20.x versions of AssemblyScript.
 * The module, @assemblyscript/wasi-shim is required to enable a WASI environment.
 * 
 * This demo is meant to showcase some abilities of WASI utilized through the AssemblyScript language.
 * It uses the latest version of AssemblyScript and will not work in older (<0.20.x) versions.
**/

// @assemblyscript/as-wasi overrides console.log to use WASI bindings.
// Print text to the terminal.
console.log("Hello World from WASI-enabled AssemblyScript utilizing WasmTime!");