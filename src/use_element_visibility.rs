use crate::core::ElementMaybeSignal;
use crate::{use_intersection_observer_with_options, UseIntersectionObserverOptions};
use default_struct_builder::DefaultBuilder;
use leptos::*;
use std::marker::PhantomData;

/// Tracks the visibility of an element within the viewport.
///
/// ## Demo
///
/// [Link to Demo](https://github.com/Synphonyte/leptos-use/tree/main/examples/use_element_visibility)
///
/// ## Usage
///
/// ```
/// # use leptos::*;
/// # use leptos::html::Div;
/// # use leptos_use::use_element_visibility;
/// #
/// # #[component]
/// # fn Demo() -> impl IntoView {
/// let el = create_node_ref::<Div>();
///
/// let is_visible = use_element_visibility(el);
///
/// view! {
///     <div node_ref=el>
///         <h1>{is_visible}</h1>
///     </div>
/// }
/// # }
/// ```
///
/// ## Server-Side Rendering
///
/// Please refer to ["Functions with Target Elements"](https://leptos-use.rs/server_side_rendering.html#functions-with-target-elements)
///
/// ## See also
///
/// * [`use_intersection_observer`]
pub fn use_element_visibility<El, T>(target: El) -> Signal<bool>
where
    El: Clone,
    El: Into<ElementMaybeSignal<T, web_sys::Element>>,
    T: Into<web_sys::Element> + Clone + 'static,
{
    use_element_visibility_with_options::<El, T, web_sys::Element, web_sys::Element>(
        target,
        UseElementVisibilityOptions::default(),
    )
}

pub fn use_element_visibility_with_options<El, T, ContainerEl, ContainerT>(
    target: El,
    options: UseElementVisibilityOptions<ContainerEl, ContainerT>,
) -> Signal<bool>
where
    El: Into<ElementMaybeSignal<T, web_sys::Element>>,
    T: Into<web_sys::Element> + Clone + 'static,
    ContainerEl: Into<ElementMaybeSignal<ContainerT, web_sys::Element>>,
    ContainerT: Into<web_sys::Element> + Clone + 'static,
{
    let (is_visible, set_visible) = create_signal(false);

    use_intersection_observer_with_options(
        (target).into(),
        move |entries, _| {
            set_visible.set(entries[0].is_intersecting());
        },
        UseIntersectionObserverOptions::default().root(options.viewport),
    );

    is_visible.into()
}

/// Options for [`use_element_visibility_with_options`].
#[derive(DefaultBuilder)]
pub struct UseElementVisibilityOptions<El, T>
where
    El: Into<ElementMaybeSignal<T, web_sys::Element>>,
    T: Into<web_sys::Element> + Clone + 'static,
{
    /// A `web_sys::Element` or `web_sys::Document` object which is an ancestor of the intended `target`,
    /// whose bounding rectangle will be considered the viewport.
    /// Any part of the target not visible in the visible area of the `root` is not considered visible.
    /// Defaults to `None` (which means the root `document` will be used).
    /// Please note that setting this to a `Some(document)` may not be supported by all browsers.
    /// See [Browser Compatibility](https://developer.mozilla.org/en-US/docs/Web/API/IntersectionObserver/IntersectionObserver#browser_compatibility)
    viewport: Option<El>,

    #[builder(skip)]
    _marker: PhantomData<T>,
}

impl Default for UseElementVisibilityOptions<web_sys::Element, web_sys::Element> {
    fn default() -> Self {
        Self {
            viewport: None,
            _marker: PhantomData,
        }
    }
}
