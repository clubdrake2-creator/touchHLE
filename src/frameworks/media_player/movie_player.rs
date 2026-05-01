/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMoviePlayerController` etc.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::foundation::{ns_string, ns_url, NSInteger};
use crate::frameworks::uikit::ui_device::UIDeviceOrientation;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, todo_objc_setter, ClassExports,
    HostObject, NSZonePtr, autorelease,
};
use crate::Environment;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct State {
    active_player: Option<id>,
    /// Various apps create or start a player and await some kind of notification, 
    /// but can't handle it if that notification happens immediately.
    pending_notifications: VecDeque<(&'static str, id, Instant)>,
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
        "MPMoviePlayerController: initWithContentURL faked for SMASH. Path: {:?}",
        ns_url::to_rust_path(env, url),
    );

    retain(env, url);
    env.objc.borrow_mut::<MPMoviePlayerControllerHostObject>(this).content_url = url;

    // Report that content is preloaded after a tiny delay
    State::get(env).pending_notifications.push_back(
        (MPMoviePlayerContentPreloadDidFinishNotification, this, Instant::now() + Duration::from_millis(100))
    );

    this
}

- (())dealloc {
    let url = env.objc.borrow::<MPMoviePlayerControllerHostObject>(this).content_url;
    release(env, url);
    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)view {
    // Create an invisible view that doesn't block touches
    let view: id = msg_class![env; UIView alloc];
    let view: id = msg![env; view initWithFrame:crate::frameworks::uikit::ui_geometry::CGRect::zero()];
    
    () = msg![env; view setUserInteractionEnabled:false];
    () = msg![env; view setBackgroundColor:msg_class![env; UIColor clearColor]];
    
    autorelease(env, view);
    view
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

    // Send the "Finished" signal after a very brief delay to let the engine initialize
    let finish_time = Instant::now() + Duration::from_millis(200);
    let notif = (MPMoviePlayerPlaybackDidFinishNotification, this, finish_time);
    State::get(env).pending_notifications.push_back(notif);
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

// Stubs for common setters
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
    let mut notifs_to_run = Vec::new();
    let pending_notifs = &mut State::get(env).pending_notifications;
    let mut i = 0;
    while i < pending_notifs.len() {
        let (name_str, object, time) = pending_notifs[i];
        if Instant::now() >= time {
            notifs_to_run.push((name_str, object));
            pending_notifs.swap_remove_back(i);
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
