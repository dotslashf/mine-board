import index from "./index.html";

// Bun HMR + TanStack Router circular import crashes with
// `null is not an object (evaluating 'import_load_client3.replaceRouteChunk')`.
Bun.serve({
  port: 1420,
  routes: {
    "/": index,
    "/dist/app.css": Bun.file("./dist/app.css"),
  },
  development: {
    hmr: false,
    console: false,
  },
});
