#[macro_export]
macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(i64);

        impl $name {
            pub fn new(value: i64) -> Self {
                Self(value)
            }

            pub fn raw(self) -> i64 {
                self.0
            }
        }

        impl From<i64> for $name {
            fn from(value: i64) -> Self {
                Self::new(value)
            }
        }

        impl From<$name> for i64 {
            fn from(value: $name) -> Self {
                value.raw()
            }
        }
    };
}

#[macro_export]
macro_rules! spawn_async {
    ($($body:tt)*) => {
        ::wasm_bindgen_futures::spawn_local(async move {
            $($body)*
        });
    };
}

#[macro_export]
macro_rules! spawn_result {
    (
        $future:expr,
        Ok($ok:pat) => $ok_body:block,
        Err($err:pat) => $err_body:block $(,)?
    ) => {
        $crate::spawn_async! {
            match $future.await {
                Ok($ok) => $ok_body,
                Err($err) => $err_body,
            }
        }
    };
}

#[macro_export]
macro_rules! match_any_view {
    ($value:expr, { $($pattern:pat $(if $guard:expr)? => $view:expr),+ $(,)? }) => {
        match $value {
            $($pattern $(if $guard)? => ($view).into_any(),)+
        }
    };
}

#[macro_export]
macro_rules! js_get {
    ($target:expr, $key:expr) => {
        ::js_sys::Reflect::get($target, &::wasm_bindgen::JsValue::from_str($key))
    };
}

#[macro_export]
macro_rules! js_set {
    ($target:expr, $key:expr, $value:expr) => {
        ::js_sys::Reflect::set($target, &::wasm_bindgen::JsValue::from_str($key), $value)
    };
}

#[macro_export]
macro_rules! js_function {
    ($target:expr, $key:expr) => {
        $crate::js_get!($target, $key)
            .and_then(|value| ::wasm_bindgen::JsCast::dyn_into::<::js_sys::Function>(value))
    };
}

#[macro_export]
macro_rules! entity_list_component {
    (
        $vis:vis fn $name:ident($items:ident: $item_ty:ty) {
            class: $class:literal,
            key: |$key_item:ident| $key:expr,
            card: $card:ident($card_prop:ident)
        }
    ) => {
        #[component]
        $vis fn $name(
            $items: ::leptos::prelude::Signal<::std::vec::Vec<$item_ty>>,
        ) -> impl ::leptos::prelude::IntoView {
            ::leptos::view! {
                <div class=$class>
                    <For
                        each=move || $items.get()
                        key=|$key_item| $key
                        children=move |$card_prop| ::leptos::view! { <$card $card_prop=$card_prop /> }
                    />
                </div>
            }
        }
    };
}
