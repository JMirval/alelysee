use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ToastKind {
    Error,
    Info,
    Success,
}

#[derive(Clone, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub title: String,
    pub body: Option<String>,
    pub kind: ToastKind,
}

#[derive(Clone)]
pub struct Toasts {
    toasts: Signal<Vec<Toast>>,
    next_id: Signal<u64>,
}

impl Toasts {
    pub fn push(&self, title: String, body: Option<String>, kind: ToastKind) -> u64 {
        let mut next_id = self.next_id;
        let id = (next_id)();
        next_id.set(id + 1);
        let toast = Toast {
            id,
            title,
            body,
            kind,
        };
        let mut toasts = self.toasts;
        toasts.with_mut(|items| items.push(toast));
        id
    }

    pub fn dismiss(&self, id: u64) {
        let mut toasts = self.toasts;
        toasts.with_mut(|items| items.retain(|toast| toast.id != id));
    }

    pub fn error(&self, title: String, body: Option<String>) {
        self.push(title, body, ToastKind::Error);
    }

    pub fn info(&self, title: String, body: Option<String>) {
        self.push(title, body, ToastKind::Info);
    }

    pub fn success(&self, title: String, body: Option<String>) {
        self.push(title, body, ToastKind::Success);
    }
}

pub fn use_toasts() -> Toasts {
    use_context::<Toasts>()
}

#[component]
pub fn ToastProvider(children: Element) -> Element {
    let toasts = use_signal(Vec::new);
    let next_id = use_signal(|| 1_u64);
    let ctx = Toasts { toasts, next_id };
    use_context_provider(|| ctx.clone());

    rsx! {
        {children}
        ToastViewport { toasts: ctx.toasts }
    }
}

#[component]
fn ToastViewport(toasts: Signal<Vec<Toast>>) -> Element {
    let items = toasts();
    rsx! {
        div { class: "toast_region", role: "status", "aria-live": "polite",
            for toast in items.iter() {
                div {
                    key: "{toast.id}",
                    class: match toast.kind {
                        ToastKind::Error => "toast toast_error",
                        ToastKind::Info => "toast toast_info",
                        ToastKind::Success => "toast toast_success",
                    },
                    div { class: "toast_content",
                        div { class: "toast_title", "{toast.title}" }
                        if let Some(body) = &toast.body {
                            div { class: "toast_body", "{body}" }
                        }
                    }
                    button {
                        class: "toast_close",
                        onclick: {
                            let id = toast.id;
                            let mut toasts = toasts;
                            move |_| {
                                toasts.with_mut(|items| items.retain(|t| t.id != id));
                            }
                        },
                        "Dismiss"
                    }
                }
            }
        }
    }
}
