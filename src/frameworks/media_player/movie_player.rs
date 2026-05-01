/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMoviePlayerController` etc.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::foundation::{ns_string, ns_url, NSInteger};
use crate::frameworks::uikit::ui_device::UIDeviceOrientation;
use crate::frameworks::core_graphics::{CGRect, CGPoint, CGSize}; 
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, todo_objc_setter, ClassExports,
    HostObject, NSZonePtr, autorelease,
};
use crate::Environment;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct State {
    pub active_player: Option<id>,
    pub pending_notifications: VecDeque<(&'static str, id, Instant)>,
}

impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.media_player.movie_player
    }
}

type MPMovieScalingMode = NSInteger;
type MPMovieControlStyle = NSInteger;

type MPMoviePlaybackState = NSInteger;
const MPMoviePlaybackStateStopped: MPMoviePlaybackState = 0;
const MPMoviePlaybackStatePlaying: MPMoviePlaybackState = 1;

pub const MPMoviePlayerPlaybackDidFinishNotification: &str =
    "MPMoviePlayerPlaybackDidFinishNotification";
pub const MPMoviePlayerContentPreloadDidFinishNotification: &str =
    "MPMoviePlayerContentPreloadDidFinishNotification";
pub const MPMoviePlayerScalingModeDidChangeNotification: &str =
    "MPMoviePlayerScalingModeDidChangeNotification";
pub const MPMoviePlayerLoadStateDidChangeNotification: &str =
    "MPMoviePlayerLoadStateDidChangeNotification";

pub const CONSTANTS: ConstantExports = &[
    (
        "_MPMoviePlayerPlaybackDidFinishNotification",
        HostConstant::NSString(MPMoviePlayerPlaybackDidFinishNotification),
    ),
    (
        "_MPMoviePlayerContentPreloadDidFinishNotification",
        HostConstant::NSString(MPMoviePlayerContentPreloadDidFinishNotification),
    ),
    (
        "_MPMoviePlayerScalingModeDidChangeNotification",
        HostConstant::NSString(MPMoviePlayerScalingModeDidChangeNotification),
    ),
    (
        "_MPMoviePlayerLoadStateDidChangeNotification",
        HostConstant::NSString(MPMoviePlayerLoadStateDidChangeNotification),
    ),
];

struct MPMoviePlayerControllerHostObject {
    content_url: id,
}
impl HostObject for MPMoviePlayerControllerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMoviePlayerController: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(MPMoviePlayerControllerHostObject {
        content_url: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithContentURL:(id)url {
    log!(
        "MPMoviePlayerController: initWithContentURL faked. Path: {:?}",
        ns_url::to_rust_path(env, url),
    );

    retain(env, url);
    env.objc.borrow_mut::<MPMoviePlayerControllerHostObject>(this).content_url = url;

    // Trigger preload notifications almost immediately
    State::get(env).pending_notifications.push_back(
        (MPMoviePlayerContentPreloadDidFinishNotification, this, Instant::now() + Duration::from_millis(10))
    );
    
    // FIX: Send LoadStateDidChange so the game knows the "video" is ready to play
    State::get(env).pending_notifications.push_back(
        (MPMoviePlayerLoadStateDidChangeNotification, this, Instant::now() + Duration::from_millis(20))
    );

    this
}

- (())dealloc {
    let url = env.objc.borrow::<MPMoviePlayerControllerHostObject>(this).content_url;
    release(env, url);
    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)view {
    // Returning nil ensures the menu UI underneath is the primary target for touches.
    log!("MPMoviePlayerController: Returning nil for view to unblock START button.");
    nil
}

- (MPMoviePlaybackState)playbackState {
    if env.framework_state.media_player.movie_player.active_player == Some(this) {
        return MPMoviePlaybackStatePlaying;
    }
    MPMoviePlaybackStateStopped
}

- (())play {
    log!("MPMoviePlayerController [HyperHLE Fix]: Faking instant playback for SMASH.");
    
    if env.framework_state.media_player.movie_player.active_player.is_none() {
        retain(env, this);
        env.framework_state.media_player.movie_player.active_player = Some(this);
    }

    // Send the "Finished" notification
    let notif = (MPMoviePlayerPlaybackDidFinishNotification, this, Instant::now() + Duration::from_millis(50));
    State::get(env).pending_notifications.push_back(notif);

    // Call stop to clear the 'active_player' state so the game knows it can proceed
    let _: () = msg![env; this stop];
}

- (())stop {
    log!("MPMoviePlayerController: stop called for {:?}", this);
    if let Some(active) = env.framework_state.media_player.movie_player.active_player {
        if active == this {
            env.framework_state.media_player.movie_player.active_player = None;
            release(env, this);
        }
    }
}

- (())setBackgroundColor:(id)color { }
- (())setScalingMode:(MPMovieScalingMode)mode { }
- (())setControlStyle:(MPMovieControlStyle)style { }
- (())setFullscreen:(bool)fullsreen { }
- (())setMovieControlMode:(NSInteger)_mode { }
- (())setOrientation:(UIDeviceOrientation)_orientation animated:(bool)_animated { }

@end

@implementation MPMoviePlayerViewController: UIViewController

- (id)initWithContentURL:(id)url {
    log!("MPMoviePlayerViewController: initWithContentURL faked.");
    this
}

- (id)moviePlayer {
    if let Some(player) = env.framework_state.media_player.movie_player.active_player {
        return player;
    }
    nil
}

@end
    
};

pub(super) fn handle_players(env: &mut Environment) {
    // 1. Force the game to stay active (Heartbeat)
    // This counters the "app-will-resign-active" logs we see in your console.
    let center: id = msg_class![env; NSNotificationCenter defaultCenter];
    let active_notif = ns_string::get_static_str(env, "UIApplicationDidBecomeActiveNotification");
    let _: () = msg![env; center postNotificationName:active_notif object:nil];

    // 2. Interaction Fix: Manually ensure the window is accepting touches.
    // If the faked movie player left the window disabled, this forces it back on.
    let app: id = msg_class![env; UIApplication sharedApplication];
    let key_window: id = msg![env; app keyWindow];
    if !key_window.is_null() {
        let _: () = msg![env; key_window setUserInteractionEnabled:true];
    }

    // 3. Process notifications that have reached their target time.
    let mut notifs_to_run = Vec::new();
    let state = State::get(env);
    
    let mut i = 0;
    while i < state.pending_notifications.len() {
        if Instant::now() >= state.pending_notifications[i].2 {
            // notification is ready
            if let Some((name_str, object, _)) = state.pending_notifications.remove(i) {
                notifs_to_run.push((name_str, object));
            }
        } else {
            i += 1;
        }
    }

    for (name_str, object) in notifs_to_run {
        let name = ns_string::get_static_str(env, name_str);
        let center: id = msg_class![env; NSNotificationCenter defaultCenter];
        let _: () = msg![env; center postNotificationName:name object:object];
    }
        }
