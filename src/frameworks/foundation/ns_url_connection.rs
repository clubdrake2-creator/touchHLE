/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `NSURLConnection`.

use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain,
    ClassExports, HostObject, NSZonePtr,
};
use crate::frameworks::foundation::ns_string;
use std::fs;
use std::path::PathBuf;

// NSError domain / code used when reporting "no network in emulator".
const NS_URL_ERROR_DOMAIN: &str = "NSURLErrorDomain";
const NS_URL_ERROR_NOT_CONNECTED_TO_INTERNET: i32 = -1009;

struct NSURLConnectionHostObject {
    delegate: id,
    cancelled: bool,
}
impl HostObject for NSURLConnectionHostObject {}

fn make_network_error(env: &mut crate::Environment) -> id {
    use crate::frameworks::foundation::ns_string::{from_rust_string, get_static_str};
    let domain = from_rust_string(env, NS_URL_ERROR_DOMAIN.to_string());
    autorelease(env, domain);
    let desc_key = get_static_str(env, "NSLocalizedDescription");
    let desc_val = from_rust_string(env, "The network connection was lost.".to_string());
    autorelease(env, desc_val);
    let user_info: id = msg_class![env; NSMutableDictionary new];
    autorelease(env, user_info);
    () = msg![env; user_info setObject:desc_val forKey:desc_key];
    let error: id = msg_class![env; NSError alloc];
    let error: id = msg![env; error initWithDomain:domain code:NS_URL_ERROR_NOT_CONNECTED_TO_INTERNET userInfo:user_info];
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

+ (bool)canHandleRequest:(id)_request {
    true
}

// MARK: - Synchronous API (Fixed for Power Rangers)

+ (id)sendSynchronousRequest:(id)request
           returningResponse:(MutPtr<id>)response_ptr
                       error:(MutPtr<id>)error_ptr {

    if request == nil { return msg_class![env; NSData data]; }

    let url_obj: id = msg![env; request URL];
    let url_str_id: id = msg![env; url_obj absoluteString];
    let url_str = ns_string::to_rust_string(env, url_str_id);

    log!("NSURLConnection: Requesting URL: {}", url_str);

    if url_str.contains("localfeed.xml") {
        let mut path = std::path::PathBuf::from(env.bundle.bundle_path().as_str());
        path.push("game/data/video/localfeed.xml"); 

        if let Ok(content) = fs::read(&path) {
            log!("NSURLConnection [HyperHLE Fix]: Feeding localfeed.xml to game engine");

            // --- FIX START: Calculate lengths OUTSIDE the macro ---
            let content_len_i32 = content.len() as i32;
            let content_len_u32 = content.len() as u32;
            let bytes_ptr = content.as_ptr() as u32;
            // --- FIX END ---

            if !response_ptr.is_null() {
                let resp: id = msg_class![env; NSURLResponse alloc];
                let resp: id = msg![env; resp initWithURL:url_obj 
                                                 MIMEType:nil 
                                    expectedContentLength:content_len_i32 
                                         textEncodingName:nil];
                
                // --- FIX: Sequential borrow to satisfy Rust compiler ---
                let autoreleased_resp = autorelease(env, resp);
                env.mem.write(response_ptr, autoreleased_resp);
            }

            if !error_ptr.is_null() { env.mem.write(error_ptr, nil); }

            let data: id = msg_class![env; NSData alloc];
            let data: id = msg![env; data initWithBytes:bytes_ptr length:content_len_u32];
            return autorelease(env, data);
        } else {
            log!("Warning: Found localfeed.xml request, but file is missing at {:?}", path);
        }
    }

    // Default behavior for other URLs
    if !response_ptr.is_null() { env.mem.write(response_ptr, nil); }
    if !error_ptr.is_null() {
        let error = make_network_error(env);
        retain(env, error);
        env.mem.write(error_ptr, error);
    }

    msg_class![env; NSData data]
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
    msg![env; this initWithRequest:request delegate:delegate startImmediately:true]
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
    this
}

- (())start {}

- (())cancel {
    env.objc.borrow_mut::<NSURLConnectionHostObject>(this).cancelled = true;
}

- (())dealloc {
    let delegate = env.objc.borrow::<NSURLConnectionHostObject>(this).delegate;
    release(env, delegate);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end
    
};
