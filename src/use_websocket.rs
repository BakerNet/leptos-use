#![cfg_attr(feature = "ssr", allow(unused_variables, unused_imports, dead_code))]

use cfg_if::cfg_if;
use core::fmt;
use leptos::{leptos_dom::helpers::TimeoutHandle, *};
use std::rc::Rc;
use std::time::Duration;

use default_struct_builder::DefaultBuilder;
use js_sys::Array;
use wasm_bindgen::{prelude::*, JsCast, JsValue};
use web_sys::{BinaryType, CloseEvent, Event, MessageEvent, WebSocket};

use crate::utils::CloneableFnWithArg;

/// Creating and managing a [Websocket](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket) connection.
///
/// ## Demo
///
/// [Link to Demo](https://github.com/Synphonyte/leptos-use/tree/main/examples/use_websocket)
///
/// ## Usage
///
/// ```
/// # use leptos::*;
/// # use leptos_use::{use_websocket, UseWebSocketReadyState, UseWebsocketReturn};
/// #
/// # #[component]
/// # fn Demo() -> impl IntoView {
/// let UseWebsocketReturn {
///     ready_state,
///     message,
///     message_bytes,
///     send,
///     send_bytes,
///     open,
///     close,
///     ..
/// } = use_websocket("wss://echo.websocket.events/".to_string());
///
/// let send_message = move |_| {
///     let m = "Hello, world!".to_string();
///     send(m.clone());
/// };
///
/// let send_byte_message = move |_| {
///     let m = b"Hello, world!\r\n".to_vec();
///     send_bytes(m.clone());
/// };
///
/// let status = move || ready_state.get().to_string();
///
/// let connected = move || ready_state.get() == UseWebSocketReadyState::Open;
///
/// let open_connection = move |_| {
///     open();
/// };
///
/// let close_connection = move |_| {
///     close();
/// };
///
/// view! {
///     <div>
///         <p>"status: " {status}</p>
///
///         <button on:click=send_message disabled=move || !connected()>"Send"</button>
///         <button on:click=send_byte_message disabled=move || !connected()>"Send bytes"</button>
///         <button on:click=open_connection disabled=connected>"Open"</button>
///         <button on:click=close_connection disabled=move || !connected()>"Close"</button>
///
///         <p>"Receive message: " {format! {"{:?}", message}}</p>
///         <p>"Receive byte message: " {format! {"{:?}", message_bytes}}</p>
///     </div>
/// }
/// # }
/// ```
///
/// ## Server-Side Rendering
///
/// On the server the returned functions amount to noops.
pub fn use_websocket(
    url: String,
) -> UseWebsocketReturn<
    impl Fn() + Clone + 'static,
    impl Fn() + Clone + 'static,
    impl Fn(String) + Clone + 'static,
    impl Fn(Vec<u8>) + Clone + 'static,
> {
    use_websocket_with_options(url, UseWebSocketOptions::default())
}

/// Version of [`use_websocket`] that takes `UseWebSocketOptions`. See [`use_websocket`] for how to use.
pub fn use_websocket_with_options(
    url: String,
    options: UseWebSocketOptions,
) -> UseWebsocketReturn<
    impl Fn() + Clone + 'static,
    impl Fn() + Clone + 'static,
    impl Fn(String) + Clone + 'static,
    impl Fn(Vec<u8>) + Clone,
> {
    let (ready_state, set_ready_state) = create_signal(UseWebSocketReadyState::Closed);
    let (message, set_message) = create_signal(None);
    let (message_bytes, set_message_bytes) = create_signal(None);
    let ws_ref: StoredValue<Option<WebSocket>> = store_value(None);

    let reconnect_limit = options.reconnect_limit.unwrap_or(3);

    let reconnect_timer_ref: StoredValue<Option<TimeoutHandle>> = store_value(None);
    let manual = options.manual;

    let reconnect_times_ref: StoredValue<u64> = store_value(0);
    let unmounted_ref = store_value(false);

    let connect_ref: StoredValue<Option<Rc<dyn Fn()>>> = store_value(None);

    cfg_if! { if #[cfg(not(feature = "ssr"))] {
        let on_open_ref = store_value(options.on_open);
        let on_message_ref = store_value(options.on_message);
        let on_message_bytes_ref = store_value(options.on_message_bytes);
        let on_error_ref = store_value(options.on_error);
        let on_close_ref = store_value(options.on_close);

        let reconnect_interval = options.reconnect_interval.unwrap_or(3 * 1000);
        let protocols = options.protocols;

        let reconnect_ref: StoredValue<Option<Rc<dyn Fn()>>> = store_value(None);
        reconnect_ref.set_value({
            let ws = ws_ref.get_value();
            Some(Rc::new(move || {
                if reconnect_times_ref.get_value() < reconnect_limit
                    && ws
                    .clone()
                    .map_or(false, |ws: WebSocket| ws.ready_state() != WebSocket::OPEN)
                {
                    reconnect_timer_ref.set_value(
                        set_timeout_with_handle(
                            move || {
                                if let Some(connect) = connect_ref.get_value() {
                                    connect();
                                    reconnect_times_ref.update_value(|current| *current += 1);
                                }
                            },
                            Duration::from_millis(reconnect_interval),
                        )
                            .ok(),
                    );
                }
            }))
        });

        connect_ref.set_value({
            let ws = ws_ref.get_value();
            let url = url;

            Some(Rc::new(move || {
                reconnect_timer_ref.set_value(None);
                {
                    if let Some(web_socket) = &ws {
                        let _ = web_socket.close();
                    }
                }

                let web_socket = {
                    protocols.as_ref().map_or_else(
                        || WebSocket::new(&url).unwrap_throw(),
                        |protocols| {
                            let array = protocols
                                .iter()
                                .map(|p| JsValue::from(p.clone()))
                                .collect::<Array>();
                            WebSocket::new_with_str_sequence(&url, &JsValue::from(&array))
                                .unwrap_throw()
                        },
                    )
                };
                web_socket.set_binary_type(BinaryType::Arraybuffer);
                set_ready_state.set(UseWebSocketReadyState::Connecting);

                // onopen handler
                {
                    let onopen_closure = Closure::wrap(Box::new(move |e: Event| {
                        if unmounted_ref.get_value() {
                            return;
                        }

                        let callback = on_open_ref.get_value();
                        callback(e);

                        set_ready_state.set(UseWebSocketReadyState::Open);
                    }) as Box<dyn FnMut(Event)>);
                    web_socket.set_onopen(Some(onopen_closure.as_ref().unchecked_ref()));
                    // Forget the closure to keep it alive
                    onopen_closure.forget();
                }

                // onmessage handler
                {
                    let onmessage_closure = Closure::wrap(Box::new(move |e: MessageEvent| {
                        if unmounted_ref.get_value() {
                            return;
                        }

                        e.data().dyn_into::<js_sys::ArrayBuffer>().map_or_else(
                            |_| {
                                e.data().dyn_into::<js_sys::JsString>().map_or_else(
                                    |_| {
                                        unreachable!("message event, received Unknown: {:?}", e.data());
                                    },
                                    |txt| {
                                        let txt = String::from(&txt);
                                        let callback = on_message_ref.get_value();
                                        callback(txt.clone());

                                        set_message.set(Some(txt));
                                    },
                                );
                            },
                            |array_buffer| {
                                let array = js_sys::Uint8Array::new(&array_buffer);
                                let array = array.to_vec();
                                let callback = on_message_bytes_ref.get_value();
                                callback(array.clone());

                                set_message_bytes.set(Some(array));
                            },
                        );
                    })
                        as Box<dyn FnMut(MessageEvent)>);
                    web_socket.set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));
                    onmessage_closure.forget();
                }
                // onerror handler
                {
                    let onerror_closure = Closure::wrap(Box::new(move |e: Event| {
                        if unmounted_ref.get_value() {
                            return;
                        }

                        if let Some(reconnect) = &reconnect_ref.get_value() {
                            reconnect();
                        }

                        let callback = on_error_ref.get_value();
                        callback(e);

                        set_ready_state.set(UseWebSocketReadyState::Closed);
                    }) as Box<dyn FnMut(Event)>);
                    web_socket.set_onerror(Some(onerror_closure.as_ref().unchecked_ref()));
                    onerror_closure.forget();
                }
                // onclose handler
                {
                    let onclose_closure = Closure::wrap(Box::new(move |e: CloseEvent| {
                        if unmounted_ref.get_value() {
                            return;
                        }

                        if let Some(reconnect) = &reconnect_ref.get_value() {
                            reconnect();
                        }

                        let callback = on_close_ref.get_value();
                        callback(e);

                        set_ready_state.set(UseWebSocketReadyState::Closed);
                    })
                        as Box<dyn FnMut(CloseEvent)>);
                    web_socket.set_onclose(Some(onclose_closure.as_ref().unchecked_ref()));
                    onclose_closure.forget();
                }

                ws_ref.set_value(Some(web_socket));
            }))
        });
    }}

    // Send text (String)
    let send = {
        Box::new(move |data: String| {
            if ready_state.get() == UseWebSocketReadyState::Open {
                if let Some(web_socket) = ws_ref.get_value() {
                    let _ = web_socket.send_with_str(&data);
                }
            }
        })
    };

    // Send bytes
    let send_bytes = move |data: Vec<u8>| {
        if ready_state.get() == UseWebSocketReadyState::Open {
            if let Some(web_socket) = ws_ref.get_value() {
                let _ = web_socket.send_with_u8_array(&data);
            }
        }
    };

    // Open connection
    let open = move || {
        reconnect_times_ref.set_value(0);
        if let Some(connect) = connect_ref.get_value() {
            connect();
        }
    };

    // Close connection
    let close = {
        reconnect_timer_ref.set_value(None);

        move || {
            reconnect_times_ref.set_value(reconnect_limit);
            if let Some(web_socket) = ws_ref.get_value() {
                let _ = web_socket.close();
            }
        }
    };

    // Open connection (not called if option `manual` is true)
    create_effect(move |_| {
        if !manual {
            open();
        }
    });

    // clean up (unmount)
    on_cleanup(move || {
        unmounted_ref.set_value(true);
        close();
    });

    UseWebsocketReturn {
        ready_state,
        message,
        message_bytes,
        ws: ws_ref.get_value(),
        open,
        close,
        send,
        send_bytes,
    }
}

/// The current state of the `WebSocket` connection.
// #[doc(cfg(feature = "websocket"))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum UseWebSocketReadyState {
    Connecting,
    Open,
    Closing,
    Closed,
}

impl fmt::Display for UseWebSocketReadyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            UseWebSocketReadyState::Connecting => write!(f, "Connecting"),
            UseWebSocketReadyState::Open => write!(f, "Open"),
            UseWebSocketReadyState::Closing => write!(f, "Closing"),
            UseWebSocketReadyState::Closed => write!(f, "Closed"),
        }
    }
}

/// Options for [`use_websocket_with_options`].
// #[doc(cfg(feature = "websocket"))]
#[derive(DefaultBuilder)]
pub struct UseWebSocketOptions {
    /// `WebSocket` connect callback.
    on_open: Box<dyn CloneableFnWithArg<Event>>,
    /// `WebSocket` message callback for text.
    on_message: Box<dyn CloneableFnWithArg<String>>,
    /// `WebSocket` message callback for binary.
    on_message_bytes: Box<dyn CloneableFnWithArg<Vec<u8>>>,
    /// `WebSocket` error callback.
    on_error: Box<dyn CloneableFnWithArg<Event>>,
    /// `WebSocket` close callback.
    on_close: Box<dyn CloneableFnWithArg<CloseEvent>>,
    /// Retry times.
    reconnect_limit: Option<u64>,
    /// Retry interval(ms).
    reconnect_interval: Option<u64>,
    /// Manually starts connection
    manual: bool,
    /// Sub protocols
    protocols: Option<Vec<String>>,
}

impl Default for UseWebSocketOptions {
    fn default() -> Self {
        Self {
            on_open: Box::new(|_| {}),
            on_message: Box::new(|_| {}),
            on_message_bytes: Box::new(|_| {}),
            on_error: Box::new(|_| {}),
            on_close: Box::new(|_| {}),
            reconnect_limit: Some(3),
            reconnect_interval: Some(3 * 1000),
            manual: false,
            protocols: Default::default(),
        }
    }
}

/// Return type of [`use_websocket`].
// #[doc(cfg(feature = "websocket"))]
#[derive(Clone)]
pub struct UseWebsocketReturn<OpenFn, CloseFn, SendFn, SendBytesFn>
where
    OpenFn: Fn() + Clone + 'static,
    CloseFn: Fn() + Clone + 'static,
    SendFn: Fn(String) + Clone + 'static,
    SendBytesFn: Fn(Vec<u8>) + Clone + 'static,
{
    /// The current state of the `WebSocket` connection.
    pub ready_state: ReadSignal<UseWebSocketReadyState>,
    /// Latest text message received from `WebSocket`.
    pub message: ReadSignal<Option<String>>,
    /// Latest binary message received from `WebSocket`.
    pub message_bytes: ReadSignal<Option<Vec<u8>>>,
    /// The `WebSocket` instance.
    pub ws: Option<WebSocket>,
    /// Opens the `WebSocket` connection
    pub open: OpenFn,
    /// Closes the `WebSocket` connection
    pub close: CloseFn,
    /// Sends `text` (string) based data
    pub send: SendFn,
    /// Sends binary data
    pub send_bytes: SendBytesFn,
}
