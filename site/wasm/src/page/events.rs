use wasm_bindgen::closure::Closure;
use web_sys::{Event, EventTarget, PageTransitionEvent, ResizeObserver};

use super::*;

thread_local! {
    static AUTO_TIMER: Cell<Option<i32>> = const {Cell::new(None)};
    static STATUS_TIMER: Cell<Option<i32>> = const {Cell::new(None)};
    static OBSERVER: RefCell<Option<ResizeObserver>> = const {RefCell::new(None)};
    static TICK: Closure<dyn FnMut()> = Closure::new(|| { if let Err(error)=sync(){report(error)} });
    static UPDATE: Closure<dyn FnMut()> = Closure::new(|| { if let Err(error)=shuffle_data(){report(error)} });
}

pub(crate) fn listen(
    target: &EventTarget,
    name: &str,
    mut handler: impl FnMut(Event) -> Result + 'static,
) -> Result {
    let callback = Closure::<dyn FnMut(Event)>::new(move |event| {
        if let Err(error) = handler(event) {
            report(error);
        }
    });
    target.add_event_listener_with_callback(name, callback.into_js_value().unchecked_ref())
}
pub(super) fn later(ms: i32, action: impl FnOnce() -> Result + 'static) -> Result {
    let callback = Closure::once_into_js(move || {
        if let Err(error) = action() {
            report(error);
        }
    });
    window().set_timeout_with_callback_and_timeout_and_arguments_0(callback.unchecked_ref(), ms)?;
    Ok(())
}
fn refresh_timers() -> Result {
    for timer in [&AUTO_TIMER, &STATUS_TIMER] {
        if let Some(id) = timer.take() {
            window().clear_interval_with_handle(id);
        }
    }
    if SUSPENDED.get() || document().hidden() {
        return Ok(());
    }
    STATUS_TIMER.set(Some(TICK.with(|cb| {
        window().set_interval_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            100,
        )
    })?));
    if AUTO.get() {
        AUTO_TIMER.set(Some(UPDATE.with(|cb| {
            window().set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                2800,
            )
        })?));
    }
    Ok(())
}
pub(super) fn set_auto(enabled: bool) -> Result {
    let reduced_motion = reduced();
    let enabled = enabled && !reduced_motion;
    AUTO.set(enabled);
    let button = el("auto-demo")?;
    button
        .unchecked_ref::<web_sys::HtmlButtonElement>()
        .set_disabled(reduced_motion);
    attr(
        &button,
        "aria-pressed",
        if enabled { "true" } else { "false" },
    )?;
    attr(
        &button,
        "title",
        if reduced_motion {
            "Automatic data updates are paused by your reduced-motion preference"
        } else {
            "Automatically load a new dataset every 2.8 seconds"
        },
    )?;
    text(
        "auto-demo",
        if enabled {
            "Pause updates"
        } else {
            "Auto-update data"
        },
    )?;
    refresh_timers()
}
pub(super) fn start_home_updates() -> Result {
    set_auto(true)?;
    later(700, || {
        if AUTO.get() && !DETAIL.get() && !SUSPENDED.get() && !document().hidden() {
            shuffle_data()?;
        }
        Ok(())
    })
}
pub(super) fn fit_controls() -> Result {
    let stage = el("chart-stage")?;
    let overflow = stage
        .query_selector(".kartis-controls")?
        .map_or(0., |controls| {
            (controls.get_bounding_client_rect().bottom()
                - stage.get_bounding_client_rect().bottom())
            .ceil()
            .max(0.)
        });
    let value = format!("{overflow}px");
    let style = html("panel-chart")?.style();
    if style.get_property_value("padding-bottom")? != value {
        style.set_property("padding-bottom", &value)?;
    }
    Ok(())
}
pub(super) fn observe_controls() -> Result {
    let stage = el("chart-stage")?;
    OBSERVER.with(|cell| -> Result {
        if let Some(observer) = cell.borrow().as_ref() {
            observer.disconnect();
            observer.observe(&stage);
            if let Some(controls) = stage.query_selector(".kartis-controls")? {
                observer.observe(&controls);
            }
        }
        Ok(())
    })?;
    fit_controls()
}
fn resized() -> Result {
    if SUSPENDED.get() {
        return Ok(());
    }
    let height = el("chart-stage")?.get_bounding_client_rect().height();
    if height > 0. && (height - STAGE_HEIGHT.get()).abs() > 0.5 {
        mount(&selection())
    } else {
        fit_controls()
    }
}
fn target(event: &Event) -> Option<Element> {
    event.target()?.dyn_into().ok()
}
pub(super) fn install() -> Result {
    let callback = Closure::<dyn FnMut()>::new(|| {
        if let Err(error) = resized() {
            report(error)
        }
    });
    let observer = ResizeObserver::new(callback.into_js_value().unchecked_ref())?;
    OBSERVER.with(|cell| cell.replace(Some(observer)));
    let doc = document();
    let window = window();
    listen(&doc, "click", |event| {
        if let Some(node) = target(&event)
            && let Some(button) = node.closest("button, a")?
        {
            controls::click(&button, &event)?;
        }
        Ok(())
    })?;
    listen(&doc, "input", |event| {
        if let Some(node) = target(&event) {
            controls::range(&node.id())?;
        }
        Ok(())
    })?;
    listen(&doc, "change", |event| {
        if let Some(node) = target(&event) {
            controls::change(&node)?;
        }
        Ok(())
    })?;
    if let Some(media) = window.match_media("(prefers-reduced-motion: reduce)")? {
        listen(&media, "change", |_| {
            set_auto(AUTO.get())?;
            sync()
        })?;
    }
    listen(&window, "pagehide", |_| {
        SUSPENDED.set(true);
        OBSERVER.with(|cell| {
            if let Some(observer) = &*cell.borrow() {
                observer.disconnect();
            }
        });
        refresh_timers()?;
        crate::stop_gallery();
        Ok(())
    })?;
    listen(&window, "pageshow", |event| {
        if event
            .dyn_ref::<PageTransitionEvent>()
            .is_some_and(PageTransitionEvent::persisted)
        {
            SUSPENDED.set(false);
            mount(&selection())?;
            refresh_timers()?;
        }
        Ok(())
    })?;
    listen(&doc, "visibilitychange", |_| refresh_timers())?;
    set_auto(false)
}
