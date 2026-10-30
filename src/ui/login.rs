use std::rc::Rc;

use dioxus::{
    logger::tracing::{info, trace},
    prelude::*,
};
use dioxus_i18n::t;

use crate::ui::{PslinkContext, navbar::Route};

const LOGIN_CSS: Asset = asset!("/assets/styling/login.css");

#[component]
pub fn LoginScreen() -> Element {
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let PslinkContext { mut user, .. } = use_context::<PslinkContext>();
    let nav = navigator();
    let mut username_field: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut password_field: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut nachricht: Signal<Option<String>> = use_signal(|| None);

    let _focus_grabber = use_resource(move || async move {
        if let Some(field) = username_field().as_ref() {
            match field.set_focus(true).await {
                Ok(_) => trace!("Username field focused"),
                Err(e) => info!("Failed to focus username field: {:?}", e),
            }
        }
    });

    if user().is_some() {
        info!("User already logged in, redirecting from login screen to links");
        nav.replace(Route::Links {});
    }

    trace!("Rendering login screen with username: {}", username);
    rsx! {
        div { class: "modal is-active",
            div { class: "modal-background" }
            div { class: "modal-card",
                header { class: "modal-card-head",
                    p { class: "modal-card-title", {t!("headline-login")} }
                }
                div { class: "modal-card-body",
                    div { class: "field is-horizontal is-wider",
                        div { class: "field-label is-normal",
                            label { class: "label", {t!("username")} }
                        }
                        div { class: "field-body",
                            div { class: "field",
                                p { class: "control",
                                    input {
                                        autofocus: true,
                                        onmounted: move |e| {
                                            username_field.set(Some(e.data()));
                                        },
                                        r#type: "text",
                                        class: "input",
                                        placeholder: t!("username"),
                                        value: "{username}",
                                        oninput: move |e| {
                                            username.set(e.value());
                                        },
                                    }
                                }
                            }
                        }
                    }
                    div { class: "field is-horizontal is-wider",
                        div { class: "field-label is-normal",
                            label { class: "label", {t!("password")} }
                        }
                        div { class: "field-body",
                            div { class: "field",
                                p { class: "control",
                                    input {
                                        r#type: "password",
                                        class: "input",
                                        placeholder: t!("password"),
                                        value: "{password}",
                                        oninput: move |e| {
                                            password.set(e.value());
                                        },
                                        onmounted: move |e| {
                                            password_field.set(Some(e.data()));
                                        },
                                    }
                                }
                            }
                        }
                    }
                    if let Some(msg) = nachricht() {
                        div { class: "notification is-danger", "{msg}" }
                    }
                }
                footer { class: "modal-card-foot is-justify-content-flex-end",
                    div { class: "buttons",
                        button {
                            class: "button is-primary",
                            onclick: move |_| {
                                info!("Login button clicked with username: {}", username());
                                async move {
                                    match crate::api::auth_api::login(username(), password()).await {
                                        Ok(u) => {
                                            user.set(Some(u));
                                            nav.push(Route::Links {});
                                        }
                                        Err(e) => {
                                            let fehlernachricht = t!("failed-login", error : e.to_string());
                                            nachricht.set(Some(fehlernachricht));
                                            info!("Failed to login: {:?}", e);
                                        }
                                    }
                                }
                            },
                            {t!("login")}
                        }
                    }
                }
            }
            document::Stylesheet { href: LOGIN_CSS }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::models::{Lang, Role, User},
        ui::translations,
    };
    use dioxus_core::{NoOpMutations, VNodeChild};
    use dioxus_i18n::{prelude::use_init_i18n, unic_langid::langid};

    #[derive(Debug, Clone, Routable, PartialEq)]
    #[rustfmt::skip]
    enum TestRoute {
        #[route("/login")]
        LoginScreen {},
        #[route("/links")]
        DummyLinks {},
        #[route("/")]
        DummyHome {},
    }

    #[component]
    fn DummyLinks() -> Element {
        rsx! { div { "links" } }
    }

    #[component]
    fn DummyHome() -> Element {
        rsx! { div { "home" } }
    }

    fn check_scope_root_anchors(scope: &dioxus_core::ScopeState, dom: &VirtualDom) {
        let Some(mounted) = scope.try_mounted_root_node() else {
            return;
        };
        for child in mounted.vnode().children() {
            if let VNodeChild::Element(el) = child {
                for anchor in el.dynamic_anchors() {
                    assert!(
                        anchor.static_path().is_root(),
                        "Root element <{}> in scope {:?} has a non-root dynamic anchor, which overwrites root_id during hydration",
                        el.tag(),
                        scope.id()
                    );
                }
            }
        }
        for anchor in mounted.vnode().dynamic_anchors() {
            for slot in anchor.nodes() {
                if let dioxus_core::DynamicNode::Component(comp) = &*slot
                    && let Some(child_scope) = comp.mounted_scope(slot, mounted, dom)
                {
                    check_scope_root_anchors(child_scope, dom);
                }
            }
        }
    }

    #[test]
    fn test_login_screen_root_hydration_anchors_and_redirect_when_logged_in() {
        #[component]
        fn FullApp() -> Element {
            use_init_i18n(|| translations::config(langid!("en-US")));
            use_context_provider(PslinkContext::default);
            rsx! {
                Router::<Route> {}
            }
        }

        // Verify `WebNavbar` / `Navbar` / `Home` root hydration anchors on initial build.
        let mut full_vdom = VirtualDom::new(FullApp);
        full_vdom.rebuild(&mut NoOpMutations);
        check_scope_root_anchors(full_vdom.base_scope(), &full_vdom);

        #[component]
        fn TestApp() -> Element {
            use_init_i18n(|| translations::config(langid!("en-US")));
            use_context_provider(PslinkContext::default);
            rsx! {
                Router::<TestRoute> {}
            }
        }

        let mut vdom = VirtualDom::new(TestApp);
        vdom.rebuild(&mut NoOpMutations);

        // Navigate to `/login` while logged out.
        vdom.in_scope(ScopeId::APP, || {
            let router = dioxus_router::root_router().expect("router should be initialized");
            router.push(TestRoute::LoginScreen {});
        });
        vdom.render_immediate(&mut NoOpMutations);

        let current_route: TestRoute = vdom.in_scope(ScopeId::APP, || {
            dioxus_router::root_router()
                .expect("router")
                .current::<TestRoute>()
        });
        assert_eq!(current_route, TestRoute::LoginScreen {});

        // Verify that `LoginScreen`'s root element only has dynamic anchors targeting itself.
        check_scope_root_anchors(vdom.base_scope(), &vdom);

        // Now simulate the user already being logged in (e.g., session loaded or existing user).
        vdom.in_scope(ScopeId::APP, || {
            let mut ctx = consume_context::<PslinkContext>();
            ctx.user.set(Some(User {
                id: 1,
                username: "alice".to_string(),
                email: "alice@example.com".to_string(),
                role: Role::Regular,
                language: Lang::EnUS,
            }));
        });

        // Process the user signal update and verify redirect to `/links`.
        vdom.render_immediate(&mut NoOpMutations);

        let redirected_route: TestRoute = vdom.in_scope(ScopeId::APP, || {
            dioxus_router::root_router()
                .expect("router")
                .current::<TestRoute>()
        });
        assert_eq!(redirected_route, TestRoute::DummyLinks {});
    }
}
