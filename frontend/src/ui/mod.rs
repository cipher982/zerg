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
        // 1. Paint if the model is flagged as dirty ----------------------
        APP_STATE.with(|state| {
            let mut st = state.borrow_mut();
            if st.dirty {
                st.draw_nodes();
                st.dirty = false;
            }

            // 2. Other per-frame work (duration ticker, etc.) -------------
            st.dispatch(Message::AnimationTick);
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
