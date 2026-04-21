use std::{ffi::c_void, ptr::null_mut};

use classicube_sys::{_NetEventsList, Event_PluginMessage, Event_Void, Event_Void_Callback};
use tracing::debug;

#[allow(non_upper_case_globals)]
#[unsafe(no_mangle)]
static mut NetEvents: _NetEventsList = _NetEventsList {
    Connected: Event_Void {
        Handlers: [None; 32],
        Objs: [null_mut(); 32],
        Count: 0,
    },
    Disconnected: Event_Void {
        Handlers: [None; 32],
        Objs: [null_mut(); 32],
        Count: 0,
    },
    PluginMessageReceived: Event_PluginMessage {
        Handlers: [None; 32],
        Objs: [null_mut(); 32],
        Count: 0,
    },
};

#[unsafe(no_mangle)]
extern "C" fn Event_Register(
    handlers: *mut Event_Void,
    obj: *mut c_void,
    handler: Event_Void_Callback,
) {
    debug!(?handler, "Event_Register");

    let handlers = unsafe { &mut *handlers };

    handlers.Handlers[handlers.Count as usize] = handler;
    handlers.Objs[handlers.Count as usize] = obj;
    handlers.Count += 1;
}

#[unsafe(no_mangle)]
extern "C" fn Event_Unregister(
    handlers: *mut Event_Void,
    obj: *mut c_void,
    handler: Event_Void_Callback,
) {
    debug!(?handler, "Event_Unregister");

    let handlers = unsafe { &mut *handlers };

    for i in 0..handlers.Count {
        #[expect(unpredictable_function_pointer_comparisons)]
        if handlers.Handlers[i as usize] == handler && handlers.Objs[i as usize] == obj {
            for j in i..handlers.Count - 1 {
                handlers.Handlers[j as usize] = handlers.Handlers[j as usize + 1];
                handlers.Objs[j as usize] = handlers.Objs[j as usize + 1];
            }

            handlers.Count -= 1;
            handlers.Handlers[handlers.Count as usize] = None;
            handlers.Objs[handlers.Count as usize] = null_mut();
            return;
        }
    }
}
