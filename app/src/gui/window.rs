//! The few things about the window the engine does not expose.

use sdl3::sys::video;

/// The first window, which is the only one this app opens.
fn window() -> *mut video::SDL_Window {
    let mut count = 0;
    // SAFETY: SDL_GetWindows returns a list it owns the allocation of, which is freed
    // right after the one pointer is read out of it; the window itself outlives the call.
    unsafe {
        let list = video::SDL_GetWindows(&mut count);
        if list.is_null() {
            return std::ptr::null_mut();
        }
        let first = if count > 0 { *list } else { std::ptr::null_mut() };
        sdl3::sys::stdinc::SDL_free(list.cast());
        first
    }
}

/// Stops the window being dragged smaller than the panel can lay itself out in.
pub fn set_minimum_size(width: i32, height: i32) {
    let window = window();
    if window.is_null() {
        return;
    }
    // SAFETY: a valid window pointer from SDL_GetWindows, used immediately.
    unsafe {
        video::SDL_SetWindowMinimumSize(window, width, height);
    }
}
