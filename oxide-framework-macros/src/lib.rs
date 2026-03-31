use proc_macro::TokenStream;

mod controller;

/// Marks an `impl` block as an Oxide controller.
///
/// Generates a [`Controller`] trait implementation that registers all
/// route-annotated methods under the given URL prefix.
///
/// # Example
///
/// ```rust,ignore
/// #[controller("/api/users")]
/// impl UserController {
///     fn new(state: &AppState) -> Self { /* ... */ }
///
///     #[get("/")]
///     async fn list(&self) -> ApiResponse<Vec<User>> { /* ... */ }
///
///     #[get("/{id}")]
///     async fn get_one(&self, Path(id): Path<u64>) -> ApiResponse<User> { /* ... */ }
/// }
/// ```
#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    controller::expand(attr.into(), item.into())
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
