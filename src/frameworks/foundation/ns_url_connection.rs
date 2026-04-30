/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSURLConnection`.

use crate::mem::MutPtr;
use crate::objc::{autorelease, id, msg, msg_class, nil, objc_classes, retain, ClassExports};
use crate::frameworks::foundation::{ns_url, ns_data, ns_string};
use std::fs;
use std::path::PathBuf;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURLConnection: NSObject

// MARK: - HyperHLE Fix: Synchronous Request Interception

+ (id)sendSynchronousRequest:(id)request
           returningResponse:(MutPtr<id>)response_ptr
                       error:(MutPtr<id>)error_ptr {
    
    if request == nil {
        if !response_ptr.is_null() { env.mem.write(response_ptr, nil); }
        return msg_class![env; NSData data];
    }

    let url: id = msg![env; request URL];
    let ns_str: id = msg![env; url absoluteString];
    let url_str = ns_string::to_rust_string(env, ns_str);

    if url_str.contains("localfeed.xml") {
        // Brute force path for Android unofficial builds
        let path = PathBuf::from("/storage/emulated/0/Android/data/org.touchhle.android.unofficial/files/touchHLE_apps/SMASH.app/localfeed.xml");

        if let Ok(content) = fs::read(&path) {
            log!("NSURLConnection [HyperHLE Fix]: Intercepted localfeed.xml");
            
            if !error_ptr.is_null() { env.mem.write(error_ptr, nil); }
            if !response_ptr.is_null() { env.mem.write(response_ptr, nil); }

            let bytes_ptr = content.as_ptr() as u32; 
            let bytes_len = content.len() as u32;

            let data: id = msg_class![env; NSData alloc];
            let data: id = msg![env; data initWithBytes:bytes_ptr length:bytes_len];
            return autorelease(env, data);
        }
    }

    if !error_ptr.is_null() { env.mem.write(error_ptr, nil); }
    if !response_ptr.is_null() { env.mem.write(response_ptr, nil); }
    msg_class![env; NSData data]
}

+ (bool)canHandleRequest:(id)_request {
    true
}

// MARK: - Standard Methods

+ (id)connectionWithRequest:(id)request
                   delegate:(id)delegate {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRequest:request delegate:delegate];
    autorelease(env, new)
}

- (id)initWithRequest:(id)request
             delegate:(id)delegate {
    msg![env; this initWithRequest:request delegate:delegate startImmediately:true]
}

- (id)initWithRequest:(id)_request
             delegate:(id)delegate
     startImmediately:(bool)start_immediately {
    if start_immediately && delegate != nil {
        retain(env, this);
        let _: () = msg![env; delegate connection:this didFailWithError:nil];
    }
    this
}

// FIX: Changed (void) to (())
- (())cancel {
    // Stub
}

@end

};
