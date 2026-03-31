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

---

# prompt.md

## Purpose

This document defines the **core philosophy, constraints, and goals** of the framework. Any AI agent or contributor must strictly adhere to this to avoid architectural drift.

---

## Core Goal

Build a **Rust-native web framework** that delivers:

* Near-zero overhead compared to Axum
* Strong conventions with minimal boilerplate
* Production-safe defaults out of the box
* Deterministic behavior under concurrency and load

This is **not** an experiment. This is a **production-oriented framework**.

---

## Non-Negotiable Principles

### 1. Performance is a Constraint, Not a Feature

* Overhead must remain within ~5–15% of raw Axum
* No abstraction that introduces unpredictable latency
* Every feature must justify its runtime cost

If a feature degrades performance significantly → reject or redesign

---

### 2. Test-First Development (MANDATORY)

Every feature must follow:

1. Write integration test
2. Write edge/abuse test
3. Validate concurrency behavior
4. Then implement

No feature is complete without:

* concurrency tests
* failure tests
* middleware interaction tests

---

### 3. Edge-Case Driven Design

We do not design for the happy path.

Every feature must be validated against:

* high concurrency (100–1000 requests)
* malformed input
* middleware interaction
* failure propagation
* real HTTP semantics (CORS, timeouts, 429, etc.)

---

### 4. Deterministic Concurrency

All behavior under concurrency must be:

* predictable
* repeatable
* race-free

If results vary under load → it is a bug

---

### 5. Middleware Correctness Over Simplicity

Middleware must:

* preserve execution order
* handle both request and response phases
* propagate errors correctly
* never break HTTP guarantees (e.g., CORS headers on errors)

Incorrect middleware composition is unacceptable

---

### 6. Developer Experience (DX) is Critical

The framework must:

* reduce boilerplate significantly vs Axum
* provide clean, intuitive APIs
* avoid exposing unnecessary generics or complexity

Target:

* working API in <20 lines

---

### 7. Convention Over Configuration

Default behavior should:

* work without setup
* include logging, error handling, and middleware

Configuration should be optional, not required

---

### 8. Compile-Time Safety Over Runtime Magic

Rust has no reflection. We use:

* procedural macros
* type system

Avoid:

* runtime registration hacks
* dynamic behavior that weakens safety

---

## Architectural Constraints

* Must remain a **thin layer over Axum**
* No tight coupling to internal Axum APIs
* Components must remain modular (routing, middleware, config)
* State must be thread-safe (Arc-based)

---

## What We Are NOT Building

* Not a full enterprise clone of Spring Boot
* Not an ORM
* Not a monolithic system
* Not a runtime-heavy framework

We focus on:

* web layer
* developer ergonomics
* correctness under load

---

## Acceptance Criteria for Any New Feature

A feature is accepted ONLY if:

* [ ] Has integration tests
* [ ] Has concurrency tests
* [ ] Has edge/failure tests
* [ ] Does not break middleware ordering
* [ ] Maintains performance budget
* [ ] Improves developer experience measurably

---

## Red Flags (Reject Immediately)

* Adds hidden runtime cost
* Breaks determinism under concurrency
* Introduces global mutable state
* Requires users to understand internal framework details
* Complicates the API without clear benefit

---

## Guiding Philosophy

> Build the simplest possible abstraction that remains correct under extreme conditions.

---

## Summary

This framework exists to prove:

* You can have **Spring Boot–level ergonomics** in Rust
* Without sacrificing **performance or safety**
* While maintaining **predictable behavior under real-world load**

Any deviation from this direction is incorrect.

