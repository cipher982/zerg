pub mod setup;
pub mod events;
pub mod main;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use crate::messages::Message;
use crate::state::APP_STATE;
use std::rc::Rc;
use std::cell::RefCell;

// Animation loop function that uses message architecture
pub fn setup_animation_loop() {
    // Create a handler function for requestAnimationFrame
    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    
    // Closure that will be called on each animation frame
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        // Dispatch an AnimationTick message instead of directly manipulating state
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.dispatch(Message::AnimationTick);
        });
        
        // Request the next animation frame
        web_sys::window()
            .expect("no global window")
            .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            .expect("request_animation_frame failed");
    }) as Box<dyn FnMut()>));
    
    // Start the animation loop
    web_sys::window()
        .expect("no global window")
        .request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())
        .expect("request_animation_frame failed");
}
