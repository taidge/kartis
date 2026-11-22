use apace_dom::Document;
use apace_traits::events::{
    ApacePointerEvent, ApacePointerId, ApaceWheelDelta, ApaceWheelEvent, ApaceWheelPhase,
    ApaceWheelSource, MouseEventButton, PointerCoords, UiEvent,
};
use glory::apace::shell::WindowApp;
use glory::style::{Length, Overflow, Style};
use glory::ui;
use kartis::{ChartConfig, chart};

#[test]
fn a_fresh_trackpad_gesture_can_change_axis_over_a_chart() {
    let mut host = WindowApp::new();
    host.renderer().set_viewport(400, 300, 1.).unwrap();
    let config = ChartConfig {
        width: 600.,
        height: 500.,
        ..Default::default()
    };
    host.mount(
        ui::view()
            .class("chart-viewport")
            .style(
                Style::new()
                    .width(Length::Px(160.))
                    .height(Length::Px(120.))
                    .overflow_x(Overflow::Auto)
                    .overflow_y(Overflow::Auto),
            )
            .fill(
                ui::view()
                    .style(
                        Style::new()
                            .width(Length::Px(config.width))
                            .height(Length::Px(config.height)),
                    )
                    .fill(chart(config.scene().unwrap())),
            ),
    )
    .unwrap();
    host.app().settle(16).unwrap();
    let offset = |host: &WindowApp| {
        host.renderer().inspect(|doc| {
            let viewport = doc.query_selector(".chart-viewport").unwrap().unwrap();
            let position = doc.get_node(viewport).unwrap().scroll_offset();
            (position.x, position.y)
        })
    };
    let coords = PointerCoords {
        page_x: 24.,
        page_y: 24.,
        client_x: 24.,
        client_y: 24.,
        screen_x: 24.,
        screen_y: 24.,
    };
    host.handle_ui_event(UiEvent::PointerMove(ApacePointerEvent {
        id: ApacePointerId::Mouse,
        is_primary: true,
        coords,
        button: MouseEventButton::Main,
        buttons: Default::default(),
        mods: Default::default(),
        details: Default::default(),
        element: Default::default(),
        active_pointers: Default::default(),
    }));
    let wheel = |dx, dy, phase, source| {
        UiEvent::Wheel(ApaceWheelEvent {
            delta: ApaceWheelDelta::Pixels(dx, dy),
            phase,
            source,
            coords,
            mods: Default::default(),
            buttons: Default::default(),
            element: Default::default(),
        })
    };
    host.handle_ui_event(wheel(
        -36.,
        -3.,
        ApaceWheelPhase::Started,
        ApaceWheelSource::Finger,
    ));
    host.handle_ui_event(wheel(
        0.,
        0.,
        ApaceWheelPhase::Ended,
        ApaceWheelSource::Finger,
    ));
    assert_eq!(offset(&host), (36., 0.));
    host.handle_ui_event(wheel(
        -2.,
        -24.,
        ApaceWheelPhase::Started,
        ApaceWheelSource::Finger,
    ));
    assert_eq!(offset(&host), (36., 24.));
}
