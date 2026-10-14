// Enable mapping before loading the bundled main process; enabling it inside
// main.js is too late for Node to cache that module's source map.
process.setSourceMapsEnabled(true)
await import("./main.js")

export {}
