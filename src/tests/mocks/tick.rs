use std::ffi::c_int;

use classicube_sys::ScheduledTaskCallback;

#[unsafe(no_mangle)]
extern "C" fn ScheduledTask_Add(_interval: f64, _callback: ScheduledTaskCallback) -> c_int {
    0
}
