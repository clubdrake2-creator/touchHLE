/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIWindow`.
//!
//! Useful resources:
//! - [Technical Q&A QA1588: Automatic orientation support for iPhone and iPad apps](https://developer.apple.com/library/archive/qa/qa1588/_index.html)
//! - [Technical Q&A QA1688: Why won't my UIViewController rotate with the device?](https://developer.apple.com/library/archive/qa/qa1688/_index.html)

use super::UIViewHostObject;
use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_graphics::cg_affine_transform::CGAffineTransform;
use crate::frameworks::core_graphics::{CGPoint, CGRect};
use crate::frameworks::foundation::ns_string;
use crate::frameworks::uikit::ui_application::{
    UIInterfaceOrientationLandscapeLeft, UIInterfaceOrientationLandscapeRight,
};
use crate::frameworks::uikit::ui_device::{
    UIDeviceOrientationLandscapeLeft, UIDeviceOrientationLandscapeRight,
};
use crate::objc::{id, msg, msg_class, msg_super, nil, objc_classes, ClassExports};

#[derive(Default)]
pub struct State {
    /// List of visible windows for internal purposes. Non-retaining!
    ///
    /// This is public because Core Animation also uses it.
    pub windows: Vec<id>,
    /// The most recent window which received `makeKeyAndVisible` message.
    /// Non-retaining!
    pub key_window: Option<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIWindow: UIView

// TODO: more?

- (id)initWithFrame:(CGRect)frame {
    let this = msg_super![env; this initWithFrame:frame];
    // Undocumented: windows seem to be hidden by default on iOS, unlike views.
    // Super call to bypass the overriden setter on this class, which would post
    // a notification.
    () = msg_super![env; this setHidden:true];

    let list = &mut env.framework_state.uikit.ui_view.ui_window.windows;
    list.push(this);
    log_dbg!(
        "New window: {:?}. New list of all windows: {:?}",
        this,
        list,
    );

    this
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    let this = msg_super![env; this initWithCoder:coder];
    // Undocumented: windows seem to be hidden by default on iOS, unlike views.
    // Super call to bypass the overriden setter on this class, which would post
    // a notification.
    () = msg_super![env; this setHidden:true];

    let screen: id = msg_class![env; UIScreen mainScreen];
    let screen_bounds: CGRect = msg![env; screen bounds];
    let current_bounds: CGRect = msg![env; this bounds];
    if current_bounds.size != screen_bounds.size {
        log_dbg!(
            "UIWindow {:?}: overriding NIB-encoded size {:?} with UIScreen.bounds size {:?}",
            this,
            current_bounds.size,
            screen_bounds.size,
        );
        () = msg![env; this setFrame:screen_bounds];
    }

    let list = &mut env.framework_state.uikit.ui_view.ui_window.windows;
    list.push(this);
    log_dbg!(
        "New window: {:?}. New list of all windows: {:?}",
        this,
        list,
    );

    this
}

- (())dealloc {
    if let Some(key_window) = env.framework_state.uikit.ui_view.ui_window.key_window {
        if key_window == this {
            env.framework_state.uikit.ui_view.ui_window.key_window = None;
        }
    }
    let list = &mut env.framework_state.uikit.ui_view.ui_window.windows;
    let idx = list.iter().position(|&w| w == this).unwrap();
    list.remove(idx);
    log_dbg!(
        "Deallocating window {:?}. New list of all windows: {:?}",
        this,
        list,
    );
    msg_super![env; this dealloc]
}

- (())layoutIfNeeded {
    log_dbg!("[(UIWindow*){:?} layoutIfNeeded]", this);
    () = msg![env; this layoutSubviews];
}

- (id)hitTest:(CGPoint)point withEvent:(id)event {
    let subviews = env.objc.borrow::<super::UIViewHostObject>(this).subviews.clone();
    for subview in subviews.into_iter().rev() {
        let hidden: bool = msg![env; subview isHidden];
        let alpha: crate::frameworks::core_graphics::CGFloat = msg![env; subview alpha];
        let interactible: bool = msg![env; subview isUserInteractionEnabled];
        if hidden || alpha < 0.01 || !interactible { continue; }
        let sub_point: CGPoint = msg![env; subview convertPoint:point fromView:this];
        let hit: id = msg![env; subview hitTest:sub_point withEvent:event];
        if hit != nil { return hit; }
    }
    this
}
    
- (())setHidden:(bool)is_hidden {
    () = msg_super![env; this setHidden:is_hidden];
    log_dbg!("[(UIWindow*){:?} setHidden:{:?}]", this, is_hidden);
}

- (())makeKeyWindow {
    env.framework_state.uikit.ui_view.ui_window.key_window = Some(this);

    let center: id = msg_class![env; NSNotificationCenter defaultCenter];
    let notif_name = ns_string::get_static_str(env, UIWindowDidBecomeKeyNotification);
    () = msg![env; center postNotificationName:notif_name object:this userInfo:nil];
}

- (bool)isKeyWindow {
    env.framework_state.uikit.ui_view.ui_window.key_window == Some(this)
}

- (())makeKeyAndVisible {
    () = msg![env; this makeKeyWindow];
    () = msg![env; this setHidden:false];
}

- (())setContentView:(id)view {
    log_dbg!("[(UIWindow*){:?} setContentView:{:?}]", this, view);
    () = msg![env; this addSubview:view];
}

- (id)contentView {
    let subviews = &env.objc.borrow::<UIViewHostObject>(this).subviews;
    subviews.first().copied().unwrap_or(nil)
}

// Support for rootViewController (iOS 4+)
- (())setRootViewController:(id)view_controller {
    log_dbg!("[(UIWindow*){:?} setRootViewController:{:?}]", this, view_controller);
    
    if view_controller != nil {
        let view: id = msg![env; view_controller view];
        
        // Link the view controller to the view's host object so it can be retrieved later
        if view != nil {
             env.objc.borrow_mut::<super::UIViewHostObject>(view).view_controller = view_controller;
        }

        () = msg![env; this addSubview:view];
    }
}

- (id)rootViewController {
    // Attempt to find the controller from the first subview's host object
    let subviews = &env.objc.borrow::<super::UIViewHostObject>(this).subviews;
    if let Some(&first_view) = subviews.first() {
        let vc = env.objc.borrow::<super::UIViewHostObject>(first_view).view_controller;
        if vc != nil {
            return vc;
        }
    }
    
    log_dbg!("[(UIWindow*){:?} rootViewController] full implementation missing/returning nil", this);
    nil
}

- (id)nextResponder {
    msg_class![env; UIApplication sharedApplication]
}

- (())addSubview:(id)view {
    log_dbg!("[(UIWindow*){:?} addSubview:{:?}] => ()", this, view);

    if view == nil || env.objc.borrow::<UIViewHostObject>(view).view_controller == nil {
        () = msg_super![env; this addSubview:view];
        return;
    }

    let vc = env.objc.borrow::<UIViewHostObject>(view).view_controller;
    () = msg![env; vc viewWillAppear:false];
    () = msg_super![env; this addSubview:view];
    () = msg![env; vc viewDidAppear:false];

    if let Some(orientation) = match env.window.as_ref().unwrap().current_rotation() {
        crate::window::DeviceOrientation::LandscapeLeft => Some(UIDeviceOrientationLandscapeLeft),
        crate::window::DeviceOrientation::LandscapeRight => Some(UIDeviceOrientationLandscapeRight),
        crate::window::DeviceOrientation::Portrait => None,
    } {
        let should = msg![env; vc shouldAutorotateToInterfaceOrientation:orientation];
        if should {
            let transform = match orientation {
                UIInterfaceOrientationLandscapeLeft => CGAffineTransform::make_rotation(-std::f32::consts::FRAC_PI_2),
                UIInterfaceOrientationLandscapeRight => CGAffineTransform::make_rotation(std::f32::consts::FRAC_PI_2),
                _ => unimplemented!(),
            };

            let window_frame: CGRect = msg![env; this frame];
            () = msg![env; view setTransform:transform];
            () = msg![env; view setFrame:window_frame];
        }
    }
}

- (CGPoint)convertPoint:(CGPoint)point fromWindow:(id)other { 
    let this_layer: id = msg![env; this layer];
    let other_layer: id = msg![env; other layer];
    msg![env; this_layer convertPoint:point fromLayer:other_layer]
}
- (CGPoint)convertPoint:(CGPoint)point toWindow:(id)other { 
    let this_layer: id = msg![env; this layer];
    let other_layer: id = msg![env; other layer];
    msg![env; this_layer convertPoint:point toLayer:other_layer]
}

@end

};

const UIWindowDidBecomeKeyNotification: &str = "UIWindowDidBecomeKeyNotification";
pub const UIKeyboardWillShowNotification: &str = "UIKeyboardWillShowNotification";
pub const UIKeyboardDidShowNotification: &str = "UIKeyboardDidShowNotification";
pub const UIKeyboardWillHideNotification: &str = "UIKeyboardWillHideNotification";
pub const UIKeyboardDidHideNotification: &str = "UIKeyboardDidHideNotification";
pub const UIKeyboardBoundsUserInfoKey: &str = "UIKeyboardBoundsUserInfoKey";

pub const CONSTANTS: ConstantExports = &[
    ("_UIWindowDidBecomeKeyNotification", HostConstant::NSString(UIWindowDidBecomeKeyNotification)),
    ("_UIKeyboardWillShowNotification", HostConstant::NSString(UIKeyboardWillShowNotification)),
    ("_UIKeyboardDidShowNotification", HostConstant::NSString(UIKeyboardDidShowNotification)),
    ("_UIKeyboardWillHideNotification", HostConstant::NSString(UIKeyboardWillHideNotification)),
    ("_UIKeyboardDidHideNotification", HostConstant::NSString(UIKeyboardDidHideNotification)),
    ("_UIKeyboardBoundsUserInfoKey", HostConstant::NSString(UIKeyboardBoundsUserInfoKey)),
    // CGFloat window level for the status bar.
    ("_UIWindowLevelStatusBar", HostConstant::Custom(|env| {
        env.mem.alloc_and_write(1000.0f32).cast_void().cast_const()
    })),
];
    
