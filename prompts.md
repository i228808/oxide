# prompts.md

## Month 2: Procedural Macros, DI, and Developer Ergonomics

### Vision

In Month 2, we focus on **developer ergonomics** and reducing boilerplate via **procedural macros**. We'll also introduce **simplified dependency injection (DI)** and enhance test-first practices for any new features.

The goal is to allow developers to declare controllers, routes, and dependencies in a clean, Spring Boot–like way, while maintaining Rust’s compile-time safety and performance.

---

## Week 1: Procedural Macros for Controllers and Routes

### Prompt 1: `#[controller]` Macro

**Explanation:**

* Annotate a struct as a controller for a route namespace.
* Automatically registers methods annotated with route macros into the app.

**Test-first style:**

* Write tests asserting that methods are correctly routed before implementing the macro.
* Example test: Send request to `/users/1` → expect correct handler called.

### Prompt 2: Route Macros (`#[get]`, `#[post]`, etc.)

**Explanation:**

* Annotate async functions to bind them to HTTP methods and paths.
* Support path params and query extraction.

**Edge test:**

* Ensure preflight OPTIONS requests are handled correctly.
* Test that missing path params give compile-time error.

### Prompt 3: Nested Controllers / Route Groups

**Explanation:**

* Support controller nesting for structured routes (`/api/v1/users`).
* Automatically prepends namespace from controller macro.

**Edge test:**

* Ensure middleware ordering still correct when nested.
* Verify rate limiter + CORS applies at top level without double-counting.

---

## Week 2: Dependency Injection (Compile-Time)

### Prompt 4: Trait-Based DI

**Explanation:**

* Allow handlers to declare dependencies via traits and generics.
* Provide default injector using App state.

**Test-first style:**

* Test that handler receives correct dependency before implementing injector.
* Edge test: Ensure panic if dependency missing.

### Prompt 5: App-State Injection

**Explanation:**

* Expose shared state (like DB pools, caches) to handlers automatically.
* Must be thread-safe (Arc + RwLock).

**Edge test:**

* Multiple concurrent requests mutate state → no race conditions.
* Handlers cannot escape reference lifetime constraints.

### Prompt 6: Constructor Injection for Controllers

**Explanation:**

* Allow controllers to declare dependencies in constructor.
* Macro should generate wiring code automatically.

**Edge test:**

* Missing dependencies fail at compile time, not runtime.
* Multiple controllers with same dependency type → no conflict.

---

## Week 3: Enhanced Middleware and Lifecycle Hooks

### Prompt 7: Before / After Request Hooks

**Explanation:**

* Macro-driven registration of hooks that run before/after handlers.
* Can modify request/response, useful for logging, metrics.

**Edge test:**

* Panic in hook does not crash server.
* Hooks respect CORS + rate limiter ordering.

### Prompt 8: Global and Controller-Level Middleware

**Explanation:**

* Apply middleware globally or at controller level.
* Ensure proper stacking and override rules.

**Test-first style:**

* Write test that global middleware executes before controller-specific middleware.
* Test that error responses still include CORS headers.

---

## Week 4: Developer Ergonomics and Test Harness

### Prompt 9: Test-First Integration for Controllers

**Explanation:**

* Use `App::into_test_server()` from Month 1.
* Ensure each new macro feature has at least one automated test.

**Edge test:**

* Preflight OPTIONS request passes with nested controllers.
* Rate limiter and timeout middleware respect macro-generated routes.
* Concurrency: 50–100 parallel requests to macro-routed endpoints → deterministic behavior.

### Prompt 10: CLI Extensions (Optional for Month 2)

**Explanation:**

* Generate controllers/routes via CLI scaffolding.
* Macro templates prepopulated for route signatures and DI injection.

**Test-first style:**

* Scaffolded files compile and run without manual edits.
* Routes work as expected when server is launched.

---

## Edge Tests Summary

1. Nested controllers + middleware ordering
2. Missing DI dependency fails at compile-time
3. Preflight OPTIONS requests through macro-routed endpoints
4. Concurrent requests to injected state → no race conditions
5. Panic in hooks or controller method → does not crash server
6. CORS + rate limiter still applies correctly after macros
7. Generated scaffolding compiles and runs immediately

---

## Deliverables by End of Month 2

* `#[controller]` + HTTP method macros fully functional
* Trait-based DI implemented and tested
* Constructor injection working for controllers
* Middleware hooks integrated with macros
* Test-first coverage for all macro features
* CLI scaffolding (optional) for controllers and routes
* Edge cases fully validated with automated tests

---

**Key Principle:**
Maintain the Month 1 philosophy:

* **Test-first development** → write tests before macro/DI implementations
* **Edge-focused validation** → every feature validated for concurrency, panic safety, and HTTP spec correctness
* **Developer experience first** → ergonomics, minimal boilerplate, predictable API
