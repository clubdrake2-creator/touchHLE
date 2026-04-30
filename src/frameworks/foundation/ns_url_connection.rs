/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `NSURLConnection`.
//!
//! This is a stub implementation that intercepts specific local file requests
//! to bypass network-dependency checks in certain games.

use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain,
    ClassExports, HostObject, NSZonePtr,
};
use crate::frameworks::foundation::{ns_url, ns_data, ns_string};
use std::fs;

// NSError domain / code used when reporting "no network in emulator".
const NS_URL_ERROR_DOMAIN: &str = "NSURLErrorDomain";
const NS_URL_ERROR_NOT_CONNECTED_TO_INTERNET: i32 = -1009;

// ---------------------------------------------------------------------------
// Host object — stores the delegate so we can call it back.
// ---------------------------------------------------------------------------

struct NSURLConnectionHostObject {
    /// `id<NSURLConnectionDelegate>` — retained while the connection is
    /// alive, released on dealloc / cancel.
    delegate: id,
    /// Whether the connection has already been cancelled / finished.
    cancelled: bool,
}
impl HostObject for NSURLConnectionHostObject {}

// ---------------------------------------------------------------------------
// Helper — build an NSError for "not connected to internet".
// ---------------------------------------------------------------------------
fn make_network_error(env: &mut crate::Environment) -> id {
    let domain = ns_string::from_rust_string(env, NS_URL_ERROR_DOMAIN.to_string());
    autorelease(env, domain);

    let desc_key = ns_string::get_static_str(env, "NSLocalizedDescription");
    let desc_val = ns_string::from_rust_string(
        env,
        "The network connection was lost. (touchHLE: networking not supported)".to_string(),
    );
    autorelease(env, desc_val);

    let user_info: id = msg_class![env; NSMutableDictionary new];
    autorelease(env, user_info);
    let _: () = msg![env; user_info setObject:desc_val forKey:desc_key];

    let error: id = msg_class![env; NSError alloc];
    let error: id = msg![env;
        error initWithDomain:domain
                        code:NS_URL_ERROR_NOT_CONNECTED_TO_INTERNET
                    userInfo:user_info];
    autorelease(env, error);
    error
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURLConnection: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::new(NSURLConnectionHostObject {
        delegate: nil,
        cancelled: false,
    });
    env.objc.alloc_object(this, host, &mut env.mem)
}

// MARK: - canHandleRequest: (class method)

+ (bool)canHandleRequest:(id)_request {
    true
}

// MARK: - Synchronous API

+ (id)sendSynchronousRequest:(id)request
           returningResponse:(MutPtr<id>)response_ptr
                       error:(MutPtr<id>)error_ptr {

    log!("NSURLConnection sendSynchronousRequest: stub called");

    if request == nil {
        if !response_ptr.is_null() { env.mem.write(response_ptr, nil); }
        let empty_data: id = msg_class![env; NSData data];
        return empty_data;
    }

    // --- INTERCEPTION FIX ---
    let url: id = msg![env; request URL];
    
    // Adjusted: use to_rust_string which is the standard name in ns_url
    let url_str = ns_url::to_rust_string(env, url);
    log!("NSURLConnection: Requesting URL: {}", url_str);

    if url_str.contains("localfeed.xml") {
        let guest_path = "/var/mobile/Applications/00000000-0000-0000-0000-000000000000/SMASH.app/localfeed.xml";
        let host_path = env.fs.borrow().get_host_path(guest_path.into());

        if let Ok(content) = fs::read(host_path) {
            log!("NSURLConnection: Successfully intercepted localfeed.xml");
            
            if !response_ptr.is_null() { env.mem.write(response_ptr, nil); }
            if !error_ptr.is_null() { env.mem.write(error_ptr, nil); }

            // Adjusted: use from_vec which is the standard name in ns_data
            return ns_data::from_vec(env, content);
        }
    }
    // --- END INTERCEPTION FIX ---

    if !response_ptr.is_null() {
        env.mem.write(response_ptr, nil);
    }

    if !error_ptr.is_null() {
        let error = make_network_error(env);
        retain(env, error);
        env.mem.write(error_ptr, error);
    }

    let empty_data: id = msg_class![env; NSData data];
    empty_data
                       }
    
// MARK: - Asynchronous API

+ (id)connectionWithRequest:(id)request
                   delegate:(id)delegate {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRequest:request delegate:delegate];
    autorelease(env, new);
    new
}

- (id)initWithRequest:(id)request
             delegate:(id)delegate {
    msg![env;
        this initWithRequest:request
                    delegate:delegate
            startImmediately:true]
}

- (id)initWithRequest:(id)request
             delegate:(id)delegate
     startImmediately:(bool)start_immediately {

    if request == nil {
        release(env, this);
        return nil;
    }

    retain(env, delegate);
    {
        let mut host = env.objc.borrow_mut::<NSURLConnectionHostObject>(this);
        host.delegate  = delegate;
        host.cancelled = false;
    }

    if start_immediately {
        log!("NSURLConnection: request will silently fail (no networking)");
    }

    this
}

// MARK: - Instance methods

- (())start {
    log!("NSURLConnection start: silently dropping");
}

- (())cancel {
    env.objc
        .borrow_mut::<NSURLConnectionHostObject>(this)
        .cancelled = true;
}

- (())dealloc {
    let delegate = env.objc
        .borrow::<NSURLConnectionHostObject>(this)
        .delegate;
    release(env, delegate);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};

