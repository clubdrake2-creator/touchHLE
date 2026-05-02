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
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports,
    HostObject, NSZonePtr,
};
use crate::Environment;
use std::collections::VecDeque;
use std::time::Instant;

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
    env.objc.borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .content_url = url;

    // Act as if loading immediately completed (Spore Origins waits
    // for this).
    State::get(env).pending_notifications.push_back(
        (MPMoviePlayerContentPreloadDidFinishNotification,
         this, Instant::now())
    );

    this
}

- (())dealloc {
    let url = env.objc.borrow::<MPMoviePlayerControllerHostObject>(this).content_url;
    release(env, url);
    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)view {
    nil // TODO
}

- (MPMoviePlaybackState)playbackState {
    MPMoviePlaybackStateStopped // TODO
}

- (())play {
    log!("MPMoviePlayerController: play called for {:?}", this);
    if env.framework_state.media_player.movie_player
        .active_player.is_none()
    {
        retain(env, this);
        env.framework_state.media_player.movie_player
            .active_player = Some(this);
    }
    // Pretend the video finishes immediately.
    State::get(env).pending_notifications.push_back(
        (MPMoviePlayerPlaybackDidFinishNotification,
         this, Instant::now())
    );
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

- (id)backgroundColor {
    msg_class![env; UIColor blackColor] // TODO
}
- (())setBackgroundColor:(id)_color {
    // TODO
}
- (())setScalingMode:(MPMovieScalingMode)_mode {
    // TODO
}
- (())setUseApplicationAudioSession:(bool)_use_session {
    // TODO
}
- (())setControlStyle:(MPMovieControlStyle)_style {
    // TODO
}
- (())setFullscreen:(bool)_fullscreen {
    // TODO
}
- (())setMovieControlMode:(NSInteger)_mode {
    // TODO
}
- (())setOrientation:(UIDeviceOrientation)_orientation
               animated:(bool)_animated {
    // TODO
}

@end

@implementation MPMoviePlayerViewController: UIViewController

- (id)initWithContentURL:(id)url {
    log!("MPMoviePlayerViewController: initWithContentURL faked.");
    release(env, this);
    nil // TODO
}

@end

};

/// For use by `NSRunLoop` via [super::handle_players]: check movie
/// players' status, send notifications if necessary.
pub(super) fn handle_players(env: &mut Environment) {
    let mut notifs_to_run = Vec::new();
    let pending_notifs =
        &mut State::get(env).pending_notifications;
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
        let center: id =
            msg_class![env; NSNotificationCenter defaultCenter];
        let _: () = msg![env; center
            postNotificationName:name object:object];
    }
}
