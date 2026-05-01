/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The UIKit framework.

use crate::objc::{id, msg, msg_class, nil};
use crate::Environment;
use std::time::Instant;

pub mod ui_accelerometer;
pub mod ui_activity_indicator_view;
pub mod ui_application;
pub mod ui_color;
pub mod ui_device;
pub mod ui_event;
pub mod ui_font;
pub mod ui_geometry;
pub mod ui_graphics;
pub mod ui_image;
pub mod ui_image_picker_controller;
pub mod ui_nib;
pub mod ui_responder;
pub mod ui_screen;
pub mod ui_screen_mode; // Added back
pub mod ui_touch;
pub mod ui_view;
pub mod ui_view_controller;

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/UIKit.framework/UIKit",
    aliases: &[],
    class_exports: &[
        ui_accelerometer::CLASSES,
        ui_activity_indicator_view::CLASSES,
        ui_application::CLASSES,
        ui_color::CLASSES,
        ui_device::CLASSES,
        ui_event::CLASSES,
        ui_font::CLASSES,
        ui_image::CLASSES,
        ui_image_picker_controller::CLASSES,
        ui_nib::CLASSES,
        ui_responder::CLASSES,
        ui_screen::CLASSES,
        ui_touch::CLASSES,
        ui_view::CLASSES,
        ui_view::ui_alert_view::CLASSES,
        ui_view::ui_control::CLASSES,
        ui_view::ui_control::ui_button::CLASSES,
        ui_view::ui_control::ui_segmented_control::CLASSES,
        ui_view::ui_control::ui_slider::CLASSES,
        ui_view::ui_control::ui_text_field::CLASSES,
        ui_view::ui_control::ui_switch::CLASSES,
        ui_view::ui_image_view::CLASSES,
        ui_view::ui_label::CLASSES,
        ui_view::ui_picker_view::CLASSES,
        ui_view::ui_scroll_view::CLASSES,
        ui_view::ui_scroll_view::ui_text_view::CLASSES,
        ui_view::ui_web_view::CLASSES,
        ui_view::ui_window::CLASSES,
        ui_view_controller::CLASSES,
        ui_view_controller::ui_navigation_controller::CLASSES,
    ],
    constant_exports: &[
        ui_application::CONSTANTS,
        ui_device::CONSTANTS,
        ui_view::ui_control::ui_text_field::CONSTANTS,
        ui_view::ui_window::CONSTANTS,
    ],
    function_exports: &[
        ui_application::FUNCTIONS,
        ui_geometry::FUNCTIONS,
        ui_graphics::FUNCTIONS,
    ],
};

#[derive(Default)]
pub struct State {
    pub ui_accelerometer: ui_accelerometer::State,
    pub ui_application: ui_application::State,
    pub ui_color: ui_color::State,
    pub ui_device: ui_device::State,
    pub ui_font: ui_font::State,
    pub ui_geometry: ui_geometry::State,     // Added back
    pub ui_graphics: ui_graphics::State,
    pub ui_image: ui_image::State,
    pub ui_screen: ui_screen::State,
    pub ui_screen_mode: ui_screen_mode::State, // Added back
    pub ui_touch: ui_touch::State,
    pub ui_view: ui_view::State,
    pub ui_responder: ui_responder::State,
}

pub fn handle_events(env: &mut Environment) -> Option<Instant> {
    use crate::window::Event;
    use crate::window::TextInputEvent;
    use crate::frameworks::foundation::ns_string;

    // --- SMASH INPUT UNBLOCK FIX ---
    let app: id = msg_class![env; UIApplication sharedApplication];
    let window: id = msg![env; app keyWindow];
    if !window.is_null() {
        // Force the window to be key and interaction-ready
        let _: () = msg![env; window makeKeyAndVisible]; 
        let _: () = msg![env; window setUserInteractionEnabled:true];
        let _: bool = msg![env; window endEditing:true]; 
    }

    loop {
        let Some(event) = env.window_mut().pop_event() else {
            break;
        };

        match event {
            Event::Quit => {
                echo!("User requested quit, exiting.");
                ui_application::exit(env);
            }
            Event::TouchesDown(..) | Event::TouchesMove(..) | Event::TouchesUp(..) => {
                ui_touch::handle_event(env, event)
            }
            Event::AppWillResignActive => {
                log!("Handling app-will-resign-active event: forcing active state.");
                
                let center: id = msg_class![env; NSNotificationCenter defaultCenter];
                
                let will_name = ns_string::get_static_str(env, "UIApplicationWillEnterForegroundNotification");
                let _: () = msg![env; center postNotificationName:will_name object:nil];

                let did_name = ns_string::get_static_str(env, "UIApplicationDidBecomeActiveNotification");
                let _: () = msg![env; center postNotificationName:did_name object:nil];

                let delegate: id = msg![env; app delegate];
                if !delegate.is_null() {
                    let _: () = msg![env; delegate applicationDidBecomeActive:app];
                }
            }
            Event::AppWillTerminate => {
                log!("Handling app-will-terminate event.");
                ui_application::exit(env);
            }
            Event::EnterDebugger => {
                if env.is_debugging_enabled() {
                    log!("Handling EnterDebugger event: entering debugger.");
                    env.enter_debugger(None);
                }
            }
            Event::TextInput(text_event) => {
                let responder = env.framework_state.uikit.ui_responder.first_responder;
                let class = msg![env; responder class];
                let ui_text_field_class = env.objc.get_known_class("UITextField", &mut env.mem);
                if !responder.is_null() && env.objc.class_is_subclass_of(class, ui_text_field_class)
                {
                    match text_event {
                        TextInputEvent::Text(text) => {
                            ui_view::ui_control::ui_text_field::handle_text(env, responder, text)
                        }
                        TextInputEvent::Backspace => {
                            ui_view::ui_control::ui_text_field::handle_backspace(env, responder)
                        }
                        TextInputEvent::Return => {
                            ui_view::ui_control::ui_text_field::handle_return(env, responder)
                        }
                    }
                }
            }
        }
    }

    ui_accelerometer::handle_accelerometer(env)
            }
                            
